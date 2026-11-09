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
    let d = bb_flasher_pb2_mspm0::device();
    let firmware = img
        .to_bytes(0..d.flash_size, None)
        .map_err(|_| Error::InvalidFirmware)?;

    if let Some(chan) = chan {
        std::thread::scope(|s| {
            let (tx, rx) = mpsc::sync_channel::<bb_flasher_pb2_mspm0::Status>(2);
            s.spawn(move || {
                while let Ok(x) = rx.recv() {
                    let _ = chan.try_send(x.into());
                }
            });

            bb_flasher_pb2_mspm0::flash(&firmware, Some(tx), persist_eeprom)
        })
    } else {
        bb_flasher_pb2_mspm0::flash(&firmware, None, persist_eeprom)
    }
}
