//! Provide functionality to flash images to sd card

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::DownloadFlashingStatus;
use crate::error::Result;
use futures::StreamExt;

pub(crate) use bb_flasher_sd::Error;

pub(crate) async fn flash<R: std::io::Read>(
    img_resolver: impl FnOnce() -> std::io::Result<(R, u64)> + Send + 'static,
    dst: PathBuf,
    chan: tokio::sync::mpsc::Sender<DownloadFlashingStatus>,
    customization: FlashingSdLinuxConfig,
) -> Result<()> {
    let cancel = Arc::new(());
    let (tx, rx) = futures::channel::mpsc::channel(20);

    let cancel_weak = Arc::downgrade(&cancel);
    let flash_thread = std::thread::spawn(move || {
        bb_flasher_sd::flash(
            img_resolver,
            &dst,
            customization.verify,
            Some(tx),
            Some(customization.customization),
            Some(cancel_weak),
        )
    });

    // Should run until tx is dropped, i.e. flasher task is done.
    // If it is aborted, then cancel should be dropped, thereby signaling the flasher task to abort
    let chan_ref = &chan;
    rx.map(Into::into)
        .for_each(|m| async move {
            let _ = chan_ref.try_send(m);
        })
        .await;

    flash_thread.join().unwrap().map_err(Into::into)
}

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

pub(crate) async fn format(dst: PathBuf) -> crate::error::Result<()> {
    tokio::task::spawn_blocking(move || bb_flasher_sd::format(Path::new(&dst)))
        .await
        .unwrap()
        .map_err(Into::into)
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
