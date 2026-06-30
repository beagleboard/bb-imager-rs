use std::sync::mpsc;

use crate::common::DownloadFlashingStatus;

use bb_flasher_pb2_mspm0::Error;

pub(crate) fn destinations() -> (String, String) {
    let d = bb_flasher_pb2_mspm0::device();
    (d.name, d.path)
}

pub(crate) fn flash(
    img: bin_file::BinFile,
    chan: Option<mpsc::SyncSender<DownloadFlashingStatus>>,
    persist_eeprom: bool,
) -> Result<(), Error> {
    std::thread::scope(|s| {
        let d = bb_flasher_pb2_mspm0::device();
        let firmware = img.to_bytes(0..d.flash_size, None).unwrap();

        let (tx, rx) = mpsc::sync_channel::<bb_flasher_pb2_mspm0::Status>(2);
        if let Some(chan) = chan {
            s.spawn(move || {
                while let Ok(s) = rx.recv() {
                    let _ = chan.try_send(s.into());
                }
            });
        }

        bb_flasher_pb2_mspm0::flash(&firmware, tx, persist_eeprom)
    })
}

impl From<bb_flasher_pb2_mspm0::Status> for DownloadFlashingStatus {
    fn from(value: bb_flasher_pb2_mspm0::Status) -> Self {
        match value {
            bb_flasher_pb2_mspm0::Status::Preparing => Self::Preparing,
            bb_flasher_pb2_mspm0::Status::Flashing(x) => Self::FlashingProgress(x),
            bb_flasher_pb2_mspm0::Status::Verifying => Self::Verifying,
        }
    }
}
