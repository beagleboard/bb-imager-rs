//! Helpers to enable flashing BeagleConnect Freedom firmware

use std::{io::Read, sync::Arc};

use crate::{error::Result, util};
pub use bb_flasher_bcf::Error;
use futures::StreamExt;

const FIRMWARE_SIZE: u32 = 704 * 1024;

fn open_firmware(mut img: crate::img::OsImage) -> Result<Vec<u8>> {
    let mut img_data = Vec::with_capacity(FIRMWARE_SIZE as usize);
    img.read_to_end(&mut img_data)
        .map_err(|_| Error::InvalidImage)?;

    match String::from_utf8(img_data) {
        Ok(x) => util::bin_file_from_str(x)
            .map_err(|_| Error::InvalidImage)?
            .to_bytes(0..(FIRMWARE_SIZE as usize), Some(0xFF))
            .map_err(|_| Error::InvalidImage.into())
            .map(|x| x.to_vec()),
        Err(e) => {
            let img_data = e.into_bytes();
            if img_data.len() != FIRMWARE_SIZE as usize {
                Err(Error::InvalidImage.into())
            } else {
                Ok(img_data)
            }
        }
    }
}

pub async fn flash(
    img: crate::img::OsImage,
    port: &str,
    chan: &tokio::sync::mpsc::Sender<crate::DownloadFlashingStatus>,
    verify: bool,
) -> Result<()> {
    let firmware = open_firmware(img)?;
    let cancle = Arc::new(());

    let (tx, rx) = futures::channel::mpsc::channel(20);

    let cancel_weak = Arc::downgrade(&cancle);
    let port_clone = port.to_string();
    let flasher_task = tokio::task::spawn_blocking(move || {
        bb_flasher_bcf::flash(&firmware, &port_clone, verify, Some(tx), Some(cancel_weak))
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
    bb_flasher_bcf::ports()
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
