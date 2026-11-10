//! A library to flash MSPM0 co-processor in [PocketBeagle 2]. It uses the kernel driver which supports
//! [Linux Firmware Upload API].
//!
//! [PocketBeagle 2]: https://www.beagleboard.org/boards/pocketbeagle-2
//! [Linux Firmware Upload API]: https://docs.kernel.org/driver-api/firmware/fw_upload.html

use std::io::{self, Read, Write};
use std::{fs::File, path::Path, sync::mpsc};

use bb_helper::status::send as chan_send;
use thiserror::Error;

const DEVICE: &str = "mspm0l1105";
const PATH: &str = "/sys/class/firmware/mspm0l1105/";
const EEPROM: &str = "/sys/bus/i2c/devices/0-0050/eeprom";
const FIRMWARE_SIZE: usize = 32 * 1024;
/// How often the driver's upload status is polled. fw_upload has no change notification.
const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

#[derive(Error, Debug)]
/// Errors for this crate
pub enum Error {
    /// Failed to open a sysfs entry.
    #[error("Failed to open {fname}.")]
    FailedToOpen {
        /// Name of the sysfs entry.
        fname: &'static str,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// Failed to read sysfs entry
    #[error("Failed to read {fname}.")]
    FailedToRead {
        /// Name of the sysfs entry.
        fname: &'static str,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// Failed to write to a sysfs entry
    #[error("Failed to write to {fname}.")]
    FailedToWrite {
        /// Name of the sysfs entry.
        fname: &'static str,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// Flashing failed
    #[error("Failed to flash at {stage}.")]
    FlashingError {
        /// Upload stage reported by the driver, or `unknown` if the driver's error could not be
        /// parsed.
        stage: String,
        /// Error code reported by the driver.
        code: String,
    },
    /// Invalid firmware
    #[error("Provided firmware is not valid.")]
    InvalidFirmware,
}

type Result<T, E = Error> = std::result::Result<T, E>;

/// Flash firmware to MSPM0. Optionally provides live [`Status`] using a channel.
///
/// [PocketBeagle 2] also uses MSPM0 as an EEPROM. Hence provide optional persistence support for
/// EEPROM contents.
///
/// [PocketBeagle 2]: https://www.beagleboard.org/boards/pocketbeagle-2
pub fn flash(
    firmware: &[u8],
    chan: Option<mpsc::SyncSender<Status>>,
    persist_eeprom: bool,
) -> Result<()> {
    if firmware.is_empty() || firmware.len() > FIRMWARE_SIZE {
        return Err(Error::InvalidFirmware);
    }

    let mut eeprom_contents = Vec::new();

    // Copy the current EEPROM contents
    if persist_eeprom {
        let mut eeprom = File::open(EEPROM).map_err(|source| Error::FailedToOpen {
            source,
            fname: "EEPROM",
        })?;
        eeprom
            .read_to_end(&mut eeprom_contents)
            .map_err(|source| Error::FailedToRead {
                source,
                fname: "EEPROM",
            })?;
    }

    flash_fw_api(Path::new(PATH), firmware, chan.as_ref())?;

    // Write back EEPROM contents
    if persist_eeprom {
        let mut eeprom = sysfs_w_open(Path::new(EEPROM)).map_err(|source| Error::FailedToOpen {
            source,
            fname: "EEPROM",
        })?;
        eeprom
            .write_all(&eeprom_contents)
            .map_err(|source| Error::FailedToWrite {
                source,
                fname: "EEPROM",
            })?;
    }

    Ok(())
}

fn sysfs_w_open(path: &Path) -> io::Result<File> {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(false)
        .open(path)
}

fn flash_fw_api(
    base: &Path,
    firmware: &[u8],
    chan: Option<&mpsc::SyncSender<Status>>,
) -> Result<()> {
    let loading_path = base.join("loading");
    let data_path = base.join("data");
    let status_path = base.join("status");
    let error_path = base.join("error");
    let remaining_size_path = base.join("remaining_size");

    let mut inp = String::new();

    // Initial firmware upload
    {
        let mut loading_file =
            sysfs_w_open(&loading_path).map_err(|source| Error::FailedToOpen {
                source,
                fname: "loading",
            })?;
        loading_file
            .write_all(b"1")
            .map_err(|source| Error::FailedToWrite {
                source,
                fname: "loading",
            })?;
        loading_file
            .flush()
            .map_err(|source| Error::FailedToWrite {
                source,
                fname: "loading",
            })?;

        let mut data_file = sysfs_w_open(&data_path).map_err(|source| Error::FailedToOpen {
            source,
            fname: "data",
        })?;
        data_file
            .write_all(firmware)
            .map_err(|source| Error::FailedToWrite {
                source,
                fname: "data",
            })?;

        loading_file
            .write_all(b"0")
            .map_err(|source| Error::FailedToWrite {
                source,
                fname: "loading",
            })?;
    }

    // Wait for flashing to finish
    loop {
        // sysfs entries cause weird stuff if kept open after a single read/write
        let mut status_file = File::open(&status_path).map_err(|source| Error::FailedToOpen {
            source,
            fname: "status",
        })?;

        inp.clear();
        status_file
            .read_to_string(&mut inp)
            .map_err(|source| Error::FailedToRead {
                source,
                fname: "status",
            })?;

        match inp.trim() {
            "idle" => break,
            "preparing" => {
                chan_send(chan, Status::Preparing);
            }
            "transferring" => {
                let mut prog = String::with_capacity(3);
                let mut size_file =
                    File::open(&remaining_size_path).map_err(|source| Error::FailedToOpen {
                        source,
                        fname: "remaining_size",
                    })?;
                size_file
                    .read_to_string(&mut prog)
                    .map_err(|source| Error::FailedToRead {
                        source,
                        fname: "remaining_size",
                    })?;

                if let Ok(p) = prog.trim().parse::<usize>() {
                    chan_send(chan, Status::Writing(upload_progress(firmware.len(), p)));
                }
            }
            "programming" => {
                chan_send(chan, Status::Verifying);
            }
            _ => {}
        }

        std::thread::sleep(POLL_INTERVAL);
    }

    // Check for error
    {
        let mut error_file = File::open(&error_path).map_err(|source| Error::FailedToOpen {
            source,
            fname: "error",
        })?;

        inp.clear();
        error_file
            .read_to_string(&mut inp)
            .map_err(|source| Error::FailedToRead {
                source,
                fname: "error",
            })?;

        let temp = inp.trim();
        match temp {
            "none" | "" => {}
            // Skipped since firmware is the same
            "preparing:firmware-invalid" => return Ok(()),
            _ => {
                let (stage, code) = temp.split_once(':').unwrap_or(("unknown", temp));

                return Err(Error::FlashingError {
                    stage: stage.to_string(),
                    code: code.to_string(),
                });
            }
        }
    }

    Ok(())
}

/// Fraction of `total` bytes sent, given the driver's `remaining` count. Clamped to `0.0..=1.0`.
fn upload_progress(total: usize, remaining: usize) -> f32 {
    total.saturating_sub(remaining) as f32 / total as f32
}

/// Get PocketBeagle 2 MSPM0 [`Device`] information.
///
/// [PocketBeagle 2]: https://www.beagleboard.org/boards/pocketbeagle-2
pub fn device() -> Device {
    Device {
        name: DEVICE.to_string(),
        path: PATH.to_string(),
        flash_size: FIRMWARE_SIZE,
    }
}

pub use bb_helper::status::Status;

/// PocketBeagle 2 MSPM0 information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    /// Device name.
    pub name: String,
    /// Path of the firmware upload sysfs directory.
    pub path: String,
    /// Maximum firmware size in bytes.
    pub flash_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    /// Build a fake firmware-upload sysfs directory. `status` is seeded with
    /// "idle" so `flash_fw_api`'s polling loop terminates after one read (any
    /// non-terminal status would spin forever against a static file), and
    /// `error` carries whatever terminal state we want to exercise.
    fn fake_sysfs(error: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::write(p.join("loading"), b"").unwrap();
        std::fs::write(p.join("data"), b"").unwrap();
        std::fs::write(p.join("status"), b"idle").unwrap();
        std::fs::write(p.join("error"), error.as_bytes()).unwrap();
        dir
    }

    #[test]
    fn flash_fw_api_writes_firmware_and_reports_success() {
        let dir = fake_sysfs("none");
        let (tx, _rx) = mpsc::sync_channel(4);
        const FW: &[u8] = b"firmware-bytes";

        flash_fw_api(dir.path(), FW, Some(&tx)).unwrap();

        // The payload is written verbatim to the `data` entry...
        assert_eq!(std::fs::read(dir.path().join("data")).unwrap(), FW);
        // ...and `loading` is toggled 1 (start) then 0 (end) on one handle.
        assert_eq!(std::fs::read(dir.path().join("loading")).unwrap(), b"10");
    }

    #[test]
    fn flash_fw_api_empty_error_is_success() {
        let dir = fake_sysfs("");
        flash_fw_api(dir.path(), b"x", None).unwrap();
    }

    #[test]
    fn flash_fw_api_firmware_invalid_is_skipped() {
        // The driver reports this when the same image is already present; the
        // flasher treats it as a successful skip, not a failure.
        let dir = fake_sysfs("preparing:firmware-invalid");
        flash_fw_api(dir.path(), b"x", None).unwrap();
    }

    #[test]
    fn flash_fw_api_surfaces_flashing_error() {
        let dir = fake_sysfs("write:0x1234");

        match flash_fw_api(dir.path(), b"x", None).unwrap_err() {
            Error::FlashingError { stage, code } => {
                assert_eq!(stage, "write");
                assert_eq!(code, "0x1234");
            }
            other => panic!("expected FlashingError, got {other:?}"),
        }
    }

    #[test]
    fn flash_fw_api_malformed_error_does_not_panic() {
        let dir = fake_sysfs("garbage");

        match flash_fw_api(dir.path(), b"x", None).unwrap_err() {
            Error::FlashingError { stage, code } => {
                assert_eq!(stage, "unknown");
                assert_eq!(code, "garbage");
            }
            other => panic!("expected FlashingError, got {other:?}"),
        }
    }

    #[test]
    fn upload_progress_is_clamped() {
        assert_eq!(upload_progress(100, 100), 0.0);
        assert_eq!(upload_progress(100, 25), 0.75);
        assert_eq!(upload_progress(100, 0), 1.0);
        // The driver reporting more than was written used to underflow.
        assert_eq!(upload_progress(100, 150), 0.0);
    }

    #[test]
    fn flash_rejects_empty_firmware() {
        // Rejected before any sysfs access, so this runs without the driver.
        assert!(matches!(
            flash(&[], None, false),
            Err(Error::InvalidFirmware)
        ));
    }

    #[test]
    fn flash_fw_api_missing_loading_entry_fails_to_open() {
        // `sysfs_w_open` uses create(false); a missing `loading` entry must
        // surface as FailedToOpen rather than silently creating the file.
        let dir = tempfile::tempdir().unwrap();

        assert!(matches!(
            flash_fw_api(dir.path(), b"x", None).unwrap_err(),
            Error::FailedToOpen {
                fname: "loading",
                ..
            }
        ));
    }
}
