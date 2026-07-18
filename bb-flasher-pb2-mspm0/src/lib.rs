//! A library to flash MSPM0 co-processor in [PocketBeagle 2]. It uses the kernel driver which support
//! [Linux Firmware Upload API].
//!
//! [PocketBeagle 2]: https://www.beagleboard.org/boards/pocketbeagle-2
//! [Linux Firmware Upload API]: https://docs.kernel.org/driver-api/firmware/fw_upload.html

use std::io::{self, Read, Write};
use std::{fs::File, path::Path, sync::mpsc};
use thiserror::Error;

const DEVICE: &str = "mspm0l1105";
const PATH: &str = "/sys/class/firmware/mspm0l1105/";
const EEPROM: &str = "/sys/bus/i2c/devices/0-0050/eeprom";
const FIRMWARE_SIZE: usize = 32 * 1024;

#[derive(Error, Debug)]
/// Errors for this crate
pub enum Error {
    /// Failed to open a sysfs entry.
    #[error("Failed to open {fname}.")]
    FailedToOpen {
        fname: &'static str,
        #[source]
        source: io::Error,
    },
    /// Failed to read sysfs entry
    #[error("Failed to read {fname}.")]
    FailedToRead {
        fname: &'static str,
        #[source]
        source: io::Error,
    },
    /// Failed to write to a sysfs entry
    #[error("Failed to write to {fname}.")]
    FailedToWrite {
        fname: &'static str,
        #[source]
        source: io::Error,
    },
    /// Flashing failed
    #[error("Failed to flash at {stage}.")]
    FlashingError { stage: String, code: String },
    /// Invalid firmware
    #[error("Provided firmware is not valid.")]
    InvalidFirmware,
}

type Result<T, E = Error> = std::result::Result<T, E>;

/// Flash firmware to MSPM0. Also provides live [`Status`] using a channel.
///
/// [PocketBeagle 2] also uses MSPM0 as an EEPROM. Hence provide optional persistance support for
/// EEPROM contents.
///
/// [PocketBeagle 2]: https://www.beagleboard.org/boards/pocketbeagle-2
pub fn flash(firmware: &[u8], chan: mpsc::SyncSender<Status>, persist_eeprom: bool) -> Result<()> {
    if firmware.len() > FIRMWARE_SIZE {
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

    flash_fw_api(Path::new(PATH), firmware, &chan)?;

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

/// Check if the proper fw upload entries are present
pub fn check() -> Result<()> {
    const FW_ENTRIES: &[&str] = &["loading", "status", "remaining_size"];

    let fw_dir = Path::new(PATH);

    for file in FW_ENTRIES {
        check_file(file, &fw_dir.join(file))?;
    }

    Ok(())
}

fn check_file(fname: &'static str, path: &Path) -> Result<()> {
    let temp = std::fs::exists(path).map_err(|source| Error::FailedToOpen { source, fname })?;

    if temp {
        Ok(())
    } else {
        Err(Error::FailedToOpen {
            fname,
            source: io::Error::new(io::ErrorKind::NotFound, "sysfs file not found"),
        })
    }
}

fn flash_fw_api(base: &Path, firmware: &[u8], chan: &mpsc::SyncSender<Status>) -> Result<()> {
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
                let _ = chan.try_send(Status::Preparing);
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
                    let _ = chan.try_send(Status::Flashing(
                        (firmware.len() - p) as f32 / firmware.len() as f32,
                    ));
                }
            }
            "programming" => {
                let _ = chan.try_send(Status::Verifying);
            }
            _ => {}
        }
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
                let resp: Vec<&str> = temp.split(':').collect();
                assert_eq!(resp.len(), 2);

                return Err(Error::FlashingError {
                    stage: resp[0].to_string(),
                    code: resp[1].to_string(),
                });
            }
        }
    }

    Ok(())
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

/// Flashing status
pub enum Status {
    Preparing,
    Flashing(f32),
    Verifying,
}

/// PocketBeagle 2 MSPM0 information.
pub struct Device {
    pub name: String,
    pub path: String,
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

        flash_fw_api(dir.path(), FW, &tx).unwrap();

        // The payload is written verbatim to the `data` entry...
        assert_eq!(std::fs::read(dir.path().join("data")).unwrap(), FW);
        // ...and `loading` is toggled 1 (start) then 0 (end) on one handle.
        assert_eq!(std::fs::read(dir.path().join("loading")).unwrap(), b"10");
    }

    #[test]
    fn flash_fw_api_empty_error_is_success() {
        let dir = fake_sysfs("");
        let (tx, _rx) = mpsc::sync_channel(1);
        flash_fw_api(dir.path(), b"x", &tx).unwrap();
    }

    #[test]
    fn flash_fw_api_firmware_invalid_is_skipped() {
        // The driver reports this when the same image is already present; the
        // flasher treats it as a successful skip, not a failure.
        let dir = fake_sysfs("preparing:firmware-invalid");
        let (tx, _rx) = mpsc::sync_channel(1);
        flash_fw_api(dir.path(), b"x", &tx).unwrap();
    }

    #[test]
    fn flash_fw_api_surfaces_flashing_error() {
        let dir = fake_sysfs("write:0x1234");
        let (tx, _rx) = mpsc::sync_channel(1);

        match flash_fw_api(dir.path(), b"x", &tx).unwrap_err() {
            Error::FlashingError { stage, code } => {
                assert_eq!(stage, "write");
                assert_eq!(code, "0x1234");
            }
            other => panic!("expected FlashingError, got {other:?}"),
        }
    }

    #[test]
    fn flash_fw_api_missing_loading_entry_fails_to_open() {
        // `sysfs_w_open` uses create(false); a missing `loading` entry must
        // surface as FailedToOpen rather than silently creating the file.
        let dir = tempfile::tempdir().unwrap();
        let (tx, _rx) = mpsc::sync_channel(1);

        assert!(matches!(
            flash_fw_api(dir.path(), b"x", &tx).unwrap_err(),
            Error::FailedToOpen {
                fname: "loading",
                ..
            }
        ));
    }
}
