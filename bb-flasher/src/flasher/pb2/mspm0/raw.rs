use futures::channel::mpsc;

use bb_flasher_pb2_mspm0::Error;

pub(crate) async fn destinations() -> (String, String) {
    let d = bb_flasher_pb2_mspm0::device();
    (d.name, d.path)
}

pub(crate) async fn flash(
    img: bin_file::BinFile,
    chan: Option<mpsc::Sender<crate::DownloadFlashingStatus>>,
    persist_eeprom: bool,
) -> Result<(), Error> {
    let d = bb_flasher_pb2_mspm0::device();
    let firmware = img.to_bytes(0..d.flash_size, None).unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::channel::<bb_flasher_pb2_mspm0::Status>(20);
    let task =
        tokio::spawn(
            async move { bb_flasher_pb2_mspm0::flash(&firmware, &tx, persist_eeprom).await },
        );

    if let Some(mut chan) = chan {
        while let Some(s) = rx.recv().await {
            let _ = chan.try_send(s.into());
        }
    }

    task.await.unwrap()
}

impl From<bb_flasher_pb2_mspm0::Status> for crate::DownloadFlashingStatus {
    fn from(value: bb_flasher_pb2_mspm0::Status) -> Self {
        match value {
            bb_flasher_pb2_mspm0::Status::Preparing => Self::Preparing,
            bb_flasher_pb2_mspm0::Status::Flashing(x) => Self::FlashingProgress(x),
            bb_flasher_pb2_mspm0::Status::Verifying => Self::Verifying,
        }
    }
}
