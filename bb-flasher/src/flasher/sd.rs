//! Flash Linux Os Images to SD Cards with optioinal post-install customization.
//!
//! Post-install customization is only available for [BeagleBoard.org] images
//!
//! [BeagleBoard.org]: https://www.beagleboard.org/

use std::{borrow::Cow, fmt::Display, path::PathBuf};
use tokio::sync::mpsc;

use crate::{BBFlasher, BBFlasherTarget, DownloadFlashingStatus};

/// SD Card
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Target(bb_flasher_sd::Device);

impl Target {
    fn destinations_internal(filter: bool) -> std::collections::HashSet<Self> {
        bb_flasher_sd::devices(filter)
            .into_iter()
            .map(Self)
            .collect()
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
        Self::destinations_internal(false)
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

    async fn destinations(filter: bool) -> std::collections::HashSet<Self> {
        Self::destinations_internal(filter)
    }

    fn identifier(&self) -> Cow<'_, str> {
        self.0.path.to_string_lossy()
    }
}

/// Linux Image post-install customization options.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FlashingSdLinuxConfig(Vec<bb_flasher_sd::Customization>);

fn sysconf_w(sysconf: &mut Vec<u8>, key: &str, value: &str) {
    sysconf.extend(key.as_bytes());
    sysconf.extend(b"=");
    sysconf.extend(value.as_bytes());
    sysconf.extend(b"\n");
}

impl FlashingSdLinuxConfig {
    pub fn sysconfig(
        hostname: Option<Box<str>>,
        timezone: Option<Box<str>>,
        keymap: Option<Box<str>>,
        user: Option<(Box<str>, Box<str>)>,
        wifi: Option<(Box<str>, Box<str>)>,
        ssh: Option<Box<str>>,
        usb_enable_dhcp: Option<bool>,
    ) -> Self {
        let mut content = Vec::<u8>::new();

        if let Some(h) = hostname {
            sysconf_w(&mut content, "hostname", &h);
        }
        if let Some(tz) = timezone {
            sysconf_w(&mut content, "timezone", &tz);
        }
        if let Some(k) = keymap {
            sysconf_w(&mut content, "keymap", &k);
        }
        if let Some((u, p)) = user {
            sysconf_w(&mut content, "user_name", &u);
            sysconf_w(&mut content, "user_password", &p);
        }
        if let Some(x) = ssh {
            sysconf_w(&mut content, "user_authorized_key", &x);
        }
        if Some(true) == usb_enable_dhcp {
            sysconf_w(&mut content, "usb_enable_dhcp", "yes");
        }

        match wifi {
            Some((ssid, psk)) => {
                sysconf_w(&mut content, "iwd_psk_file", &format!("{ssid}.psk"));

                Self(vec![bb_flasher_sd::Customization {
                    partition: bb_flasher_sd::ParitionType::Boot,
                    content: vec![
                        (
                            "sysconf.txt".to_string().into(),
                            content.into_boxed_slice().into(),
                        ),
                        (
                            format!("services/{ssid}.psk").into(),
                            format!("[Security]\nPassphrase={psk}\n\n[Settings]\nAutoConnect=true")
                                .into_boxed_str()
                                .into_boxed_bytes()
                                .into(),
                        ),
                    ],
                }])
            }
            None => Self(vec![bb_flasher_sd::Customization {
                partition: bb_flasher_sd::ParitionType::Boot,
                content: vec![(
                    "sysconf.txt".to_string().into(),
                    content.into_boxed_slice().into(),
                )],
            }]),
        }
    }

    pub fn generic_file(file_name: Box<str>, file_content: Box<str>) -> Self {
        Self(vec![bb_flasher_sd::Customization {
            partition: bb_flasher_sd::ParitionType::Boot,
            content: vec![(file_name, file_content.into_boxed_bytes().into())],
        }])
    }

    pub const fn none() -> Self {
        Self(Vec::new())
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
    async fn flash(self, _: Option<mpsc::Sender<DownloadFlashingStatus>>) -> anyhow::Result<()> {
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
pub struct Flasher<I, B> {
    img: I,
    bmap: Option<B>,
    dst: bb_flasher_sd::Destination,
    customization: FlashingSdLinuxConfig,
    cancel: Option<tokio_util::sync::CancellationToken>,
}

impl<I, B> Flasher<I, B> {
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
            dst: bb_flasher_sd::Destination::SdCard(dst.0.path.into_boxed_path()),
            customization,
            cancel,
        }
    }

    pub fn with_file_dest(
        img: I,
        bmap: Option<B>,
        dst: PathBuf,
        customization: FlashingSdLinuxConfig,
        cancel: Option<tokio_util::sync::CancellationToken>,
    ) -> Self {
        Self {
            img,
            bmap,
            dst: bb_flasher_sd::Destination::File(dst.into_boxed_path()),
            customization,
            cancel,
        }
    }
}

impl<I> Flasher<I, std::future::Ready<std::io::Result<Box<str>>>> {
    pub fn without_bmap(
        img: I,
        dst: Target,
        customization: FlashingSdLinuxConfig,
        cancel: Option<tokio_util::sync::CancellationToken>,
    ) -> Self {
        Self::new(img, None, dst, customization, cancel)
    }
}

impl<I, B> BBFlasher for Flasher<I, B>
where
    I: Future<Output = std::io::Result<(crate::OsImage, u64)>> + Send + 'static,
    B: Future<Output = std::io::Result<Box<str>>> + Send + 'static,
{
    async fn flash(self, chan: Option<mpsc::Sender<DownloadFlashingStatus>>) -> anyhow::Result<()> {
        let customization = self.customization.0;
        let dst = self.dst;

        if let Some(chan) = chan {
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
                dst,
                Some(tx),
                customization,
                self.cancel,
            )
            .await;

            t.abort();

            resp
        } else {
            bb_flasher_sd::flash(self.img, self.bmap, dst, None, customization, self.cancel).await
        }
        .map_err(Into::into)
    }
}
