//! A library to flash MSPM0 co-processor in [PocketBeagle 2]. It uses the kernel driver which support
//! [Linux Firmware Upload API].
//!
//! [PocketBeagle 2]: https://www.beagleboard.org/boards/pocketbeagle-2
//! [Linux Firmware Upload API]: https://docs.kernel.org/driver-api/firmware/fw_upload.html

use std::{io, path::Path};
use thiserror::Error;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};

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
pub async fn flash(
    firmware: &[u8],
    chan: &tokio::sync::mpsc::Sender<Status>,
    persist_eeprom: bool,
) -> Result<()> {
    if firmware.len() > FIRMWARE_SIZE {
        return Err(Error::InvalidFirmware);
    }

    let mut eeprom_contents = Vec::new();

    // Copy the current EEPROM contents
    if persist_eeprom {
        let mut eeprom = File::open(EEPROM)
            .await
            .map_err(|source| Error::FailedToOpen {
                source,
                fname: "EEPROM",
            })?;
        eeprom
            .read_to_end(&mut eeprom_contents)
            .await
            .map_err(|source| Error::FailedToRead {
                source,
                fname: "EEPROM",
            })?;
    }

    flash_fw_api(Path::new(PATH), firmware, chan).await?;

    // Write back EEPROM contents
    if persist_eeprom {
        let mut eeprom =
            sysfs_w_open(Path::new(EEPROM))
                .await
                .map_err(|source| Error::FailedToOpen {
                    source,
                    fname: "EEPROM",
                })?;
        eeprom
            .write_all(&eeprom_contents)
            .await
            .map_err(|source| Error::FailedToWrite {
                source,
                fname: "EEPROM",
            })?;
    }

    Ok(())
}

async fn sysfs_w_open(path: &Path) -> io::Result<File> {
    tokio::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(false)
        .open(path)
        .await
}

/// Check if the proper fw upload entries are present
pub async fn check() -> Result<()> {
    const FW_ENTRIES: &[&str] = &["loading", "status", "remaining_size"];

    let fw_dir = Path::new(PATH);

    for file in FW_ENTRIES {
        check_file(file, &fw_dir.join(file)).await?;
    }

    Ok(())
}

async fn check_file(fname: &'static str, path: &Path) -> Result<()> {
    let temp = tokio::fs::try_exists(path)
        .await
        .map_err(|source| Error::FailedToOpen { source, fname })?;

    if temp {
        Ok(())
    } else {
        Err(Error::FailedToOpen {
            fname,
            source: io::Error::new(io::ErrorKind::NotFound, "sysfs file not found"),
        })
    }
}

async fn flash_fw_api(
    base: &Path,
    firmware: &[u8],
    chan: &tokio::sync::mpsc::Sender<Status>,
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
            sysfs_w_open(&loading_path)
                .await
                .map_err(|source| Error::FailedToOpen {
                    source,
                    fname: "loading",
                })?;
        loading_file
            .write_all(b"1")
            .await
            .map_err(|source| Error::FailedToWrite {
                source,
                fname: "loading",
            })?;
        loading_file
            .flush()
            .await
            .map_err(|source| Error::FailedToWrite {
                source,
                fname: "loading",
            })?;

        let mut data_file =
            sysfs_w_open(&data_path)
                .await
                .map_err(|source| Error::FailedToOpen {
                    source,
                    fname: "data",
                })?;
        data_file
            .write_all(firmware)
            .await
            .map_err(|source| Error::FailedToWrite {
                source,
                fname: "data",
            })?;

        loading_file
            .write_all(b"0")
            .await
            .map_err(|source| Error::FailedToWrite {
                source,
                fname: "loading",
            })?;
    }

    // Wait for flashing to finish
    loop {
        // sysfs entries cause weird stuff if kept open after a single read/write
        let mut status_file =
            File::open(&status_path)
                .await
                .map_err(|source| Error::FailedToOpen {
                    source,
                    fname: "status",
                })?;

        inp.clear();
        status_file
            .read_to_string(&mut inp)
            .await
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
                let mut size_file = File::open(&remaining_size_path).await.map_err(|source| {
                    Error::FailedToOpen {
                        source,
                        fname: "remaining_size",
                    }
                })?;
                size_file
                    .read_to_string(&mut prog)
                    .await
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
        let mut error_file =
            File::open(&error_path)
                .await
                .map_err(|source| Error::FailedToOpen {
                    source,
                    fname: "error",
                })?;

        inp.clear();
        error_file
            .read_to_string(&mut inp)
            .await
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

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
/// Flashing status
pub enum Status {
    Preparing,
    Flashing(f32),
    Verifying,
}

/// PocketBeagle 2 MSPM0 information.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "zvariant", derive(zvariant::Type))]
pub struct Device {
    pub name: String,
    pub path: String,
    pub flash_size: usize,
}
