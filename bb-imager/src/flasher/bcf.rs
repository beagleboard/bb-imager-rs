//! Helpers to enable flashing BeagleConnect Freedom firmware

use std::{io::Read, sync::Arc};

use crate::BBFlasher;
pub use bb_flasher_bcf::cc1352p7::Error;
use futures::StreamExt;

pub fn possible_devices() -> std::collections::HashSet<crate::Destination> {
    bb_flasher_bcf::cc1352p7::ports()
        .into_iter()
        .map(crate::Destination::port)
        .collect()
}

#[derive(Clone, Debug)]
pub struct FlashingBcfConfig {
    pub verify: bool,
}

impl FlashingBcfConfig {
    pub fn update_verify(mut self, verify: bool) -> Self {
        self.verify = verify;
        self
    }
}

impl Default for FlashingBcfConfig {
    fn default() -> Self {
        Self { verify: true }
    }
}

impl From<bb_flasher_bcf::Status> for crate::DownloadFlashingStatus {
    fn from(value: bb_flasher_bcf::Status) -> Self {
        match value {
            bb_flasher_bcf::Status::Preparing => Self::Preparing,
            bb_flasher_bcf::Status::Flashing(x) => Self::FlashingProgress(x),
            bb_flasher_bcf::Status::Verifying => Self::Verifying,
        }
    }
}

pub struct BeagleConnectFreedom<I: crate::img::ImageFile> {
    img: I,
    port: String,
    customization: FlashingBcfConfig,
}

impl<I> BeagleConnectFreedom<I>
where
    I: crate::img::ImageFile,
{
    pub const fn new(img: I, port: String, customization: FlashingBcfConfig) -> Self {
        Self {
            img,
            port,
            customization,
        }
    }
}

impl<I> BBFlasher for BeagleConnectFreedom<I>
where
    I: crate::img::ImageFile + Clone + Sync,
{
    async fn flash(
        self,
        chan: Option<futures::channel::mpsc::Sender<crate::DownloadFlashingStatus>>,
    ) -> std::io::Result<()> {
        let cancle = Arc::new(());

        let cancel_weak = Arc::downgrade(&cancle);
        let port = self.port;
        let verify = self.customization.verify;
        let img = {
            let mut img = crate::img::OsImage::open(self.img, chan.clone()).await?;
            let mut data = Vec::new();
            img.read_to_end(&mut data)?;
            data
        };

        let flasher_task = if let Some(chan) = chan {
            let (tx, rx) = futures::channel::mpsc::channel(20);
            let flasher_task = tokio::task::spawn_blocking(move || {
                bb_flasher_bcf::cc1352p7::flash(&img, &port, verify, Some(tx), Some(cancel_weak))
            });

            // Should run until tx is dropped, i.e. flasher task is done.
            // If it is aborted, then cancel should be dropped, thereby signaling the flasher task to abort
            let _ = rx.map(Into::into).map(Ok).forward(chan).await;

            flasher_task
        } else {
            tokio::task::spawn_blocking(move || {
                bb_flasher_bcf::cc1352p7::flash(&img, &port, verify, None, Some(cancel_weak))
            })
        };

        flasher_task.await.unwrap().map_err(|e| match e {
            Error::IoError(error) => error,
            _ => std::io::Error::other(e),
        })
    }
}
