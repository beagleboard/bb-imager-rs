use std::collections::HashSet;

pub use bb_imager_flasher_pb2_mspm0::Error;

pub fn possible_devices() -> std::collections::HashSet<crate::Destination> {
    let d = bb_imager_flasher_pb2_mspm0::device();
    HashSet::from([crate::Destination::file(d.name, d.path)])
}

pub async fn flash(
    img: bin_file::BinFile,
    chan: &tokio::sync::mpsc::Sender<crate::DownloadFlashingStatus>,
    persist_eeprom: bool,
) -> crate::error::Result<()> {
    let d = bb_imager_flasher_pb2_mspm0::device();
    let firmware = img.to_bytes(0..d.flash_size, None).unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<bb_imager_flasher_pb2_mspm0::Status>(20);

    let res = tokio::spawn(async move {
        bb_imager_flasher_pb2_mspm0::flash(&firmware, &tx, persist_eeprom)
            .await
            .map_err(Into::into)
    });

    while let Some(s) = rx.recv().await {
        let _ = chan.try_send(s.into());
    }

    res.await.unwrap()
}

impl From<bb_imager_flasher_pb2_mspm0::Status> for crate::DownloadFlashingStatus {
    fn from(value: bb_imager_flasher_pb2_mspm0::Status) -> Self {
        match value {
            bb_imager_flasher_pb2_mspm0::Status::Preparing => Self::Preparing,
            bb_imager_flasher_pb2_mspm0::Status::Flashing(x) => Self::FlashingProgress(x),
            bb_imager_flasher_pb2_mspm0::Status::Verifying => Self::Verifying,
        }
    }
}
