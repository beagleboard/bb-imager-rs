//! Helpers to enable flashing BeagleConnect Freedom MSP430 firmware

use futures::StreamExt;

use crate::error::Result;

pub use bb_flasher_bcf::msp430::Error;

pub(crate) async fn flash(
    img: Vec<u8>,
    dst: &std::ffi::CStr,
    chan: &tokio::sync::mpsc::Sender<crate::DownloadFlashingStatus>,
) -> Result<()> {
    let (tx, rx) = futures::channel::mpsc::channel(20);

    let dst_owned = dst.to_owned();
    let flasher_task = tokio::task::spawn_blocking(move || {
        bb_flasher_bcf::msp430::flash(&img, &dst_owned, Some(tx))
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
    bb_flasher_bcf::msp430::devices()
        .into_iter()
        .map(crate::Destination::hidraw)
        .collect()
}
