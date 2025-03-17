//! Helpers to enable flashing BeagleConnect Freedom MSP430 firmware

use std::io::Read;

use futures::StreamExt;

use crate::BBFlasher;

pub use bb_flasher_bcf::msp430::Error;

pub fn possible_devices() -> std::collections::HashSet<crate::Destination> {
    bb_flasher_bcf::msp430::devices()
        .into_iter()
        .map(crate::Destination::hidraw)
        .collect()
}

pub struct Msp430<I: crate::img::ImageFile> {
    img: I,
    port: std::ffi::CString,
}

impl<I> Msp430<I>
where
    I: crate::img::ImageFile,
{
    pub const fn new(img: I, port: std::ffi::CString) -> Self {
        Self { img, port }
    }
}

impl<I> BBFlasher for Msp430<I>
where
    I: crate::img::ImageFile,
{
    async fn flash(
        self,
        chan: Option<futures::channel::mpsc::Sender<crate::DownloadFlashingStatus>>,
    ) -> std::io::Result<()> {
        let dst = self.port;
        let img = {
            let mut img = crate::img::OsImage::open(self.img, chan.clone()).await?;
            let mut data = Vec::new();
            img.read_to_end(&mut data)?;
            data
        };

        let flasher_task = if let Some(chan) = chan {
            let (tx, rx) = futures::channel::mpsc::channel(20);
            let flasher_task = tokio::task::spawn_blocking(move || {
                bb_flasher_bcf::msp430::flash(&img, &dst, Some(tx))
            });

            // Should run until tx is dropped, i.e. flasher task is done.
            // If it is aborted, then cancel should be dropped, thereby signaling the flasher task to abort
            let _ = rx.map(Into::into).map(Ok).forward(chan).await;

            flasher_task
        } else {
            tokio::task::spawn_blocking(move || bb_flasher_bcf::msp430::flash(&img, &dst, None))
        };

        flasher_task.await.unwrap().map_err(std::io::Error::other)
    }
}
