//! Flash Linux Os Images to SD Cards with optioinal post-install customization.
//!
//! Post-install customization is only available for [BeagleBoard.org] images
//!
//! [BeagleBoard.org]: https://www.beagleboard.org/

use std::{borrow::Cow, fmt::Display, path::PathBuf};

use crate::{BBFlasher, BBFlasherTarget, DownloadFlashingStatus, Resolvable};

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

    pub fn path(&self) -> &std::path::Path {
        &self.0.path
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

    fn identifier(&self) -> Cow<'_, str> {
        self.0.path.to_string_lossy()
    }
}

/// Linux Image post-install customization options. Only work on BeagleBoard.org images.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FlashingSdLinuxConfig {
    customization: Option<bb_flasher_sd::Customization>,
}

impl FlashingSdLinuxConfig {
    pub const fn sysconfig(
        hostname: Option<Box<str>>,
        timezone: Option<Box<str>>,
        keymap: Option<Box<str>>,
        user: Option<(Box<str>, Box<str>)>,
        wifi: Option<(Box<str>, Box<str>)>,
        ssh: Option<Box<str>>,
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
    ) -> anyhow::Result<()> {
        let p = self.0;
        bb_flasher_sd::format(p.as_path()).await.map_err(Into::into)
    }
}

/// Flasher of flashing Os Images to SD Card
///
/// # Supported Images
///
/// - img: Raw images
/// - xz: Xz compressed raw images
#[derive(Debug, Clone)]
pub struct Flasher<I: Resolvable, B: Resolvable> {
    img: I,
    bmap: Option<B>,
    dst: PathBuf,
    customization: FlashingSdLinuxConfig,
    cancel: Option<tokio_util::sync::CancellationToken>,
}

impl<I, B> Flasher<I, B>
where
    I: Resolvable,
    B: Resolvable,
{
    pub fn new(
        img: I,
        bmap: Option<B>,
        dst: Target,
        customization: FlashingSdLinuxConfig,
        cancel: Option<tokio_util::sync::CancellationToken>,
    ) -> Self {
        Self {
            img,
            bmap,
            dst: dst.0.path,
            customization,
            cancel,
        }
    }
}

impl<I, B> BBFlasher for Flasher<I, B>
where
    I: Resolvable<ResolvedType = (crate::OsImage, u64)> + Send + 'static,
    B: Resolvable<ResolvedType = Box<str>> + Send + 'static,
{
    async fn flash(
        self,
        chan: Option<futures::channel::mpsc::Sender<DownloadFlashingStatus>>,
    ) -> anyhow::Result<()> {
        let customization = self.customization.customization;
        let dst = self.dst;

        if let Some(mut chan) = chan {
            let (tx, mut rx) = tokio::sync::mpsc::channel(2);

            let t = tokio::spawn(async move {
                // Should run until tx is dropped, i.e. flasher task is done.
                // If it is aborted, then cancel should be dropped, thereby signaling the flasher task to abort
                while let Some(x) = rx.recv().await {
                    let _ = chan.try_send(if x == 0.0 {
                        DownloadFlashingStatus::Preparing
                    } else {
                        DownloadFlashingStatus::FlashingProgress(x)
                    });
                }
            });

            let resp = bb_flasher_sd::flash(
                self.img,
                self.bmap,
                dst.into(),
                Some(tx),
                customization,
                self.cancel,
            )
            .await;

            t.abort();

            resp
        } else {
            bb_flasher_sd::flash(
                self.img,
                self.bmap,
                dst.into(),
                None,
                customization,
                self.cancel,
            )
            .await
        }
        .map_err(Into::into)
    }
}
