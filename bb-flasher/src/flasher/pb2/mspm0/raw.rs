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

    bb_flasher_pb2_mspm0::flash(&firmware, chan, persist_eeprom)
}
