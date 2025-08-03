//! Flash Linux Os Images to SD Cards with optioinal post-install customization.
//!
//! Post-install customization is only available for [BeagleBoard.org] images
//!
//! [BeagleBoard.org]: https://www.beagleboard.org/

use std::{fmt::Display, io::Read, path::PathBuf, sync::Arc};

use crate::{BBFlasher, BBFlasherTarget, DownloadFlashingStatus, ImageFile};
use futures::StreamExt;

use bb_flasher_sd::Error;

/// SD Card
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Target(bb_flasher_sd::Device);

impl Target {
    fn destinations_internal() -> std::collections::HashSet<Self> {
        bb_flasher_sd::devices().into_iter().map(Self).collect()
    }

    /// SD Card size in bytes
    pub const fn size(&self) -> u64 {
        self.0.size
    }
}

impl Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.name.fmt(f)
    }
}

impl TryFrom<PathBuf> for Target {
    type Error = std::io::Error;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        Self::destinations_internal()
            .into_iter()
            .find(|x| x.0.path == value)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "SD Card target not found",
            ))
    }
}

impl BBFlasherTarget for Target {
    const FILE_TYPES: &[&str] = &["img", "xz"];

    async fn destinations() -> std::collections::HashSet<Self> {
        Self::destinations_internal()
    }

    fn is_destination_selectable() -> bool {
        true
    }

    fn path(&self) -> &std::path::Path {
        &self.0.path
    }
}

/// Linux Image post-install customization options. Only work on BeagleBoard.org images.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FlashingSdLinuxConfig {
    customization: Option<bb_flasher_sd::Customization>,
}

impl FlashingSdLinuxConfig {
    pub const fn sysconfig(
        hostname: Option<String>,
        timezone: Option<String>,
        keymap: Option<String>,
        user: Option<(String, String)>,
        wifi: Option<(String, String)>,
        ssh: Option<String>,
        usb_enable_dhcp: Option<bool>,
    ) -> Self {
        Self {
            customization: Some(bb_flasher_sd::Customization::Sysconf(
                bb_flasher_sd::SysconfCustomization {
                    hostname,
                    timezone,
                    keymap,
                    user,
                    wifi,
                    ssh,
                    usb_enable_dhcp,
                },
            )),
        }
    }

    pub const fn none() -> Self {
        Self {
            customization: None,
        }
    }
}

/// Flasher to format SD Cards
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FormatFlasher(PathBuf);

impl FormatFlasher {
    pub fn new(p: Target) -> Self {
        Self(p.0.path)
    }
}

impl BBFlasher for FormatFlasher {
    async fn flash(
        self,
        _: Option<futures::channel::mpsc::Sender<DownloadFlashingStatus>>,
    ) -> std::io::Result<()> {
        let p = self.0;
        tokio::task::spawn_blocking(move || bb_flasher_sd::format(p.as_path()))
            .await
            .unwrap()
            .map_err(|e| match e {
                Error::IoError(error) => error,
                _ => std::io::Error::other(e.to_string()),
            })
    }
}

/// Flasher of flashing Os Images to SD Card
///
/// # Supported Images
///
/// - img: Raw images
/// - xz: Xz compressed raw images
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Flasher<I: ImageFile, B: ImageFile> {
    img: I,
    bmap: Option<B>,
    dst: PathBuf,
    customization: FlashingSdLinuxConfig,
}

impl<I, B> Flasher<I, B>
where
    I: ImageFile,
    B: ImageFile,
{
    pub fn new(img: I, bmap: Option<B>, dst: Target, customization: FlashingSdLinuxConfig) -> Self {
        Self {
            img,
            bmap,
            dst: dst.0.path,
            customization,
        }
    }
}

impl<I, B> BBFlasher for Flasher<I, B>
where
    I: ImageFile + Send + 'static,
    B: ImageFile + Send + 'static,
{
    async fn flash(
        self,
        chan: Option<futures::channel::mpsc::Sender<DownloadFlashingStatus>>,
    ) -> std::io::Result<()> {
        let chan_clone = chan.clone();
        let img = self.img;
        let bmap = self.bmap;

        let img_resolver = move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .enable_io()
                .build()
                .unwrap();
            let chan_clone_1 = chan_clone.clone();
            let bmap = match bmap {
                Some(x) => {
                    let mut f =
                        rt.block_on(
                            async move { crate::img::OsImage::open(x, chan_clone_1).await },
                        )?;
                    let mut data = String::new();
                    f.read_to_string(&mut data)?;
                    Some(data)
                }
                None => None,
            };
            let img =
                rt.block_on(async move { crate::img::OsImage::open(img, chan_clone).await })?;
            let img_size = img.size();

            Ok((img, img_size, bmap))
        };

        let cancel = Arc::new(());
        let cancel_weak = Arc::downgrade(&cancel);

        let customization = self.customization.customization;
        let dst = self.dst;

        let flash_thread = if let Some(chan) = chan {
            let (tx, rx) = futures::channel::mpsc::channel(20);

            let flash_thread = std::thread::spawn(move || {
                bb_flasher_sd::flash(
                    img_resolver,
                    &dst,
                    Some(tx),
                    customization,
                    Some(cancel_weak),
                )
            });

            // Should run until tx is dropped, i.e. flasher task is done.
            // If it is aborted, then cancel should be dropped, thereby signaling the flasher task to abort
            let _ = rx
                .map(|x| {
                    if x == 0.0 {
                        DownloadFlashingStatus::Preparing
                    } else {
                        DownloadFlashingStatus::FlashingProgress(x)
                    }
                })
                .map(Ok)
                .forward(chan)
                .await;

            flash_thread
        } else {
            std::thread::spawn(move || {
                bb_flasher_sd::flash(img_resolver, &dst, None, customization, Some(cancel_weak))
            })
        };

        flash_thread.join().unwrap().map_err(|e| match e {
            Error::IoError(error) => error,
            _ => std::io::Error::other(e),
        })
    }
}
