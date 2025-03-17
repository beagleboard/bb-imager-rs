//! Provide functionality to flash images to sd card

use std::{path::PathBuf, sync::Arc};

use crate::{BBFlasher, DownloadFlashingStatus};
use futures::StreamExt;

pub(crate) use bb_flasher_sd::Error;

pub fn destinations() -> std::collections::HashSet<crate::Destination> {
    bb_flasher_sd::devices()
        .into_iter()
        .map(|x| crate::Destination::sd_card(x.name, x.size, x.path))
        .collect()
}

#[derive(Clone, Debug)]
pub struct FlashingSdLinuxConfig {
    verify: bool,
    customization: bb_flasher_sd::Customization,
}

impl FlashingSdLinuxConfig {
    pub const fn update_verify(mut self, verify: bool) -> Self {
        self.verify = verify;
        self
    }

    pub const fn verify(&self) -> bool {
        self.verify
    }

    pub fn update_hostname(mut self, hostname: Option<String>) -> Self {
        self.customization.hostname = hostname;
        self
    }

    pub fn hostname(&self) -> Option<&str> {
        self.customization.hostname.as_deref()
    }

    pub fn update_timezone(mut self, timezone: Option<String>) -> Self {
        self.customization.timezone = timezone;
        self
    }

    pub fn timezone(&self) -> Option<&str> {
        self.customization.timezone.as_deref()
    }

    pub fn update_keymap(mut self, k: Option<String>) -> Self {
        self.customization.keymap = k;
        self
    }

    pub fn keymap(&self) -> Option<&str> {
        self.customization.keymap.as_deref()
    }

    pub fn update_user(mut self, v: Option<(String, String)>) -> Self {
        self.customization.user = v;
        self
    }

    pub fn user(&self) -> Option<(&str, &str)> {
        self.customization
            .user
            .as_ref()
            .map(|(x, y)| (x.as_str(), y.as_str()))
    }

    pub fn update_wifi(mut self, v: Option<(String, String)>) -> Self {
        self.customization.wifi = v;
        self
    }

    pub fn wifi(&self) -> Option<(&str, &str)> {
        self.customization
            .wifi
            .as_ref()
            .map(|(x, y)| (x.as_str(), y.as_str()))
    }
}

impl Default for FlashingSdLinuxConfig {
    fn default() -> Self {
        Self {
            verify: true,
            customization: Default::default(),
        }
    }
}

impl From<bb_flasher_sd::Status> for crate::DownloadFlashingStatus {
    fn from(value: bb_flasher_sd::Status) -> Self {
        match value {
            bb_flasher_sd::Status::Preparing => Self::Preparing,
            bb_flasher_sd::Status::Flashing(x) => Self::FlashingProgress(x),
            bb_flasher_sd::Status::Verifying(x) => Self::VerifyingProgress(x),
        }
    }
}

pub struct LinuxSdFormat(PathBuf);

impl LinuxSdFormat {
    pub const fn new(p: PathBuf) -> Self {
        Self(p)
    }
}

impl BBFlasher for LinuxSdFormat {
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

pub struct LinuxSd<I: crate::img::ImageFile> {
    img: I,
    dst: PathBuf,
    customization: FlashingSdLinuxConfig,
}

impl<I> LinuxSd<I>
where
    I: crate::img::ImageFile,
{
    pub const fn new(img: I, dst: PathBuf, customization: FlashingSdLinuxConfig) -> Self {
        Self {
            img,
            dst,
            customization,
        }
    }
}

impl<I> BBFlasher for LinuxSd<I>
where
    I: crate::img::ImageFile + Send + 'static,
{
    async fn flash(
        self,
        chan: Option<futures::channel::mpsc::Sender<DownloadFlashingStatus>>,
    ) -> std::io::Result<()> {
        let chan_clone = chan.clone();
        let img = self.img;

        let img_resolver = move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .enable_io()
                .build()
                .unwrap();
            let img =
                rt.block_on(async move { crate::img::OsImage::open(img, chan_clone).await })?;
            let img_size = img.size();

            Ok((img, img_size))
        };

        let cancel = Arc::new(());
        let cancel_weak = Arc::downgrade(&cancel);

        let verify = self.customization.verify;
        let customization = self.customization.customization;
        let dst = self.dst;

        let flash_thread = if let Some(chan) = chan {
            let (tx, rx) = futures::channel::mpsc::channel(20);

            let flash_thread = std::thread::spawn(move || {
                bb_flasher_sd::flash(
                    img_resolver,
                    &dst,
                    verify,
                    Some(tx),
                    Some(customization),
                    Some(cancel_weak),
                )
            });

            // Should run until tx is dropped, i.e. flasher task is done.
            // If it is aborted, then cancel should be dropped, thereby signaling the flasher task to abort
            let _ = rx
                .map(DownloadFlashingStatus::from)
                .map(Ok)
                .forward(chan)
                .await;

            flash_thread
        } else {
            std::thread::spawn(move || {
                bb_flasher_sd::flash(
                    img_resolver,
                    &dst,
                    verify,
                    None,
                    Some(customization),
                    Some(cancel_weak),
                )
            })
        };

        flash_thread.join().unwrap().map_err(|e| match e {
            Error::IoError(error) => error,
            _ => std::io::Error::other(e),
        })
    }
}
