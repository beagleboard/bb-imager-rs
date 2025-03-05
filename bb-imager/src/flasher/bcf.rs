//! Helpers to enable flashing BeagleConnect Freedom firmware

use std::sync::Arc;

use crate::error::Result;
pub use bb_flasher_bcf::cc1352p7::Error;
use futures::StreamExt;

pub async fn flash(
    img: Vec<u8>,
    port: &str,
    chan: &tokio::sync::mpsc::Sender<crate::DownloadFlashingStatus>,
    verify: bool,
) -> Result<()> {
    let cancle = Arc::new(());

    let (tx, rx) = futures::channel::mpsc::channel(20);

    let cancel_weak = Arc::downgrade(&cancle);
    let port_clone = port.to_string();
    let flasher_task = tokio::task::spawn_blocking(move || {
        bb_flasher_bcf::cc1352p7::flash(&img, &port_clone, verify, Some(tx), Some(cancel_weak))
    });

    // Should run until tx is dropped, i.e. flasher task is done.
    // If it is aborted, then cancel should be dropped, thereby signaling the flasher task to abort
    rx.map(Into::into)
        .for_each(|m| async move {
            let _ = chan.try_send(m);
        })
        .await;

    flasher_task.await.unwrap().map_err(Into::into)
}

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
