//! Integration tests for the parts of the public API that do not require the
//! PocketBeagle 2 firmware-upload sysfs interface (and hence real hardware).
//!
//! The sysfs flashing paths need a `flash_fw_api(&Path, ..)` seam plus a
//! temp-dir fixture to test without hardware; that is deferred. Covered here:
//! the oversize-firmware guard (which returns before touching sysfs) and the
//! pure `device()` accessor.

use std::sync::mpsc;

use bb_flasher_pb2_mspm0::{Error, Status, device, flash};

#[test]
fn flash_rejects_oversize_firmware() {
    // flash_size is 32 KiB; one byte over must be rejected. The size check runs
    // before any sysfs/EEPROM access, so this is safe to run without hardware.
    let flash_size = device().flash_size;
    let firmware = vec![0u8; flash_size + 1];
    let (tx, _rx) = mpsc::sync_channel::<Status>(1);

    let result = flash(&firmware, tx, false);
    assert!(
        matches!(result, Err(Error::InvalidFirmware)),
        "expected InvalidFirmware, got {result:?}"
    );
}

#[test]
fn device_reports_expected_metadata() {
    let device = device();
    assert_eq!(device.name, "mspm0l1105");
    assert_eq!(device.path, "/sys/class/firmware/mspm0l1105/");
    assert_eq!(device.flash_size, 32 * 1024);
}
