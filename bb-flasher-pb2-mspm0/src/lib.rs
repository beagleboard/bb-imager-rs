//! A library to flash MSPM0 co-processor in [PocketBeagle 2]. It uses the kernel driver which support
//! [Linux Firmware Upload API].
//!
//! [PocketBeagle 2]: https://www.beagleboard.org/boards/pocketbeagle-2
//! [Linux Firmware Upload API]: https://docs.kernel.org/driver-api/firmware/fw_upload.html

use std::path::Path;
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
    #[error("Failed to open {0}")]
    FailedToOpen(&'static str),
    /// Failed to read sysfs entry
    #[error("Failed to read {0}")]
    FailedToRead(&'static str),
    /// Failed to write to a sysfs entry
    #[error("Failed to write to {0}")]
    FailedToWrite(&'static str),
    /// Failed to Seek to start for a sysfs entry
    #[error("Failed to seek {0}")]
    FailedToSeek(&'static str),
    /// Flashing failed
    #[error("Failed to flash at {stage} due to {code}")]
    FlashingError { stage: String, code: String },
    /// Invalid firmware
    #[error("Invalid firmware")]
    InvalidFirmware,
}

impl Error {
    /// Get user-friendly error message suitable for GUI display
    pub fn user_message(&self) -> String {
        match self {
            Error::FailedToOpen("loading") | Error::FailedToOpen("data") | Error::FailedToOpen("status") => {
                "Cannot access PocketBeagle 2 firmware interface. Please ensure:\n\
                 - Your board is properly connected via USB\n\
                 - You have the required permissions (try running with sudo)\n\
                 - The kernel driver is loaded".to_string()
            }
            Error::FailedToOpen("EEPROM") => {
                "Cannot access EEPROM on PocketBeagle 2. The board may not be properly detected.".to_string()
            }
            Error::FailedToRead(entry) | Error::FailedToWrite(entry) | Error::FailedToSeek(entry) => {
                format!("Communication error with PocketBeagle 2 ({}). Try reconnecting the board.", entry)
            }
            Error::FlashingError { stage, code } => {
                format!("Flashing failed during {} stage: {}. Check the firmware file and try again.", stage, code)
            }
            Error::InvalidFirmware => {
                "The firmware file is invalid or corrupted. Please download a valid firmware file.".to_string()
            }
            Error::FailedToOpen(other) => {
                format!("Cannot access system resource: {}. Check permissions and connections.", other)
            }
        }
    }
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
        tracing::error!(
            "Firmware size {} bytes exceeds maximum {} bytes",
            firmware.len(),
            FIRMWARE_SIZE
        );
        return Err(Error::InvalidFirmware);
    }

    let mut eeprom_contents = Vec::new();

    // Copy the current EEPROM contents
    if persist_eeprom {
        tracing::debug!("Reading EEPROM contents for preservation");
        let mut eeprom = File::open(EEPROM)
            .await
            .map_err(|e| {
                tracing::error!("Failed to open EEPROM at {}: {}", EEPROM, e);
                Error::FailedToOpen("EEPROM")
            })?;
        eeprom
            .read_to_end(&mut eeprom_contents)
            .await
            .map_err(|e| {
                tracing::error!("Failed to read EEPROM contents: {}", e);
                Error::FailedToRead("EEPROM")
            })?;
        tracing::debug!("Read {} bytes from EEPROM", eeprom_contents.len());
    }

    flash_fw_api(Path::new(PATH), firmware, chan).await?;

    // Write back EEPROM contents
    if persist_eeprom {
        tracing::debug!("Restoring EEPROM contents");
        let mut eeprom = sysfs_w_open(Path::new(EEPROM))
            .await
            .map_err(|e| {
                tracing::error!("Failed to open EEPROM for writing: {}", e);
                Error::FailedToOpen("EEPROM")
            })?;
        eeprom
            .write_all(&eeprom_contents)
            .await
            .map_err(|e| {
                tracing::error!("Failed to restore EEPROM contents: {}", e);
                Error::FailedToWrite("EEPROM")
            })?;
        tracing::debug!("Successfully restored EEPROM contents");
    }

    Ok(())
}

async fn sysfs_w_open(path: &Path) -> std::io::Result<File> {
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

async fn check_file(name: &'static str, path: &Path) -> Result<()> {
    let temp = tokio::fs::try_exists(path)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check existence of {} at {:?}: {}", name, path, e);
            Error::FailedToOpen(name)
        })?;

    if temp {
        Ok(())
    } else {
        tracing::error!("Sysfs entry {} does not exist at {:?}", name, path);
        Err(Error::FailedToOpen(name))
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
        tracing::info!("Starting firmware upload ({} bytes)", firmware.len());
        let mut loading_file = sysfs_w_open(&loading_path)
            .await
            .map_err(|e| {
                tracing::error!("Failed to open loading sysfs entry at {:?}: {}", loading_path, e);
                Error::FailedToOpen("loading")
            })?;
        loading_file
            .write_all(b"1")
            .await
            .map_err(|e| {
                tracing::error!("Failed to write '1' to loading entry: {}", e);
                Error::FailedToWrite("loading")
            })?;
        loading_file
            .flush()
            .await
            .map_err(|e| {
                tracing::error!("Failed to flush loading entry: {}", e);
                Error::FailedToWrite("loading")
            })?;

        let mut data_file = sysfs_w_open(&data_path)
            .await
            .map_err(|e| {
                tracing::error!("Failed to open data sysfs entry at {:?}: {}", data_path, e);
                Error::FailedToOpen("data")
            })?;
        data_file
            .write_all(firmware)
            .await
            .map_err(|e| {
                tracing::error!("Failed to write firmware data: {}", e);
                Error::FailedToWrite("data")
            })?;

        loading_file
            .write_all(b"0")
            .await
            .map_err(|e| {
                tracing::error!("Failed to write '0' to loading entry: {}", e);
                Error::FailedToWrite("loading")
            })?;
        tracing::debug!("Firmware upload initiated successfully");
    }

    // Wait for flashing to finish
    tracing::debug!("Monitoring flashing progress");
    loop {
        // sysfs entries cause weird stuff if kept open after a single read/write
        let mut status_file = File::open(&status_path)
            .await
            .map_err(|e| {
                tracing::error!("Failed to open status sysfs entry: {}", e);
                Error::FailedToOpen("status")
            })?;

        inp.clear();
        status_file
            .read_to_string(&mut inp)
            .await
            .map_err(|e| {
                tracing::error!("Failed to read status: {}", e);
                Error::FailedToRead("status")
            })?;

        match inp.trim() {
            "idle" => {
                tracing::info!("Flashing completed successfully");
                break;
            }
            "preparing" => {
                tracing::debug!("Status: preparing");
                let _ = chan.try_send(Status::Preparing);
            }
            "transferring" => {
                let mut prog = String::with_capacity(3);
                let mut size_file = File::open(&remaining_size_path)
                    .await
                    .map_err(|e| {
                        tracing::warn!("Failed to open remaining_size: {}", e);
                        Error::FailedToOpen("remaining_size")
                    })?;
                size_file
                    .read_to_string(&mut prog)
                    .await
                    .map_err(|e| {
                        tracing::warn!("Failed to read remaining_size: {}", e);
                        Error::FailedToRead("remaining_size")
                    })?;

                if let Ok(p) = prog.trim().parse::<usize>() {
                    let progress_pct = (firmware.len() - p) as f32 / firmware.len() as f32;
                    tracing::debug!("Flashing progress: {:.1}%", progress_pct * 100.0);
                    let _ = chan.try_send(Status::Flashing(progress_pct));
                }
            }
            "programming" => {
                tracing::debug!("Status: programming (verifying)");
                let _ = chan.try_send(Status::Verifying);
            }
            other => {
                tracing::debug!("Unknown status: {}", other);
            }
        }
    }

    // Check for error
    {
        let mut error_file = File::open(&error_path)
            .await
            .map_err(|e| {
                tracing::error!("Failed to open error sysfs entry: {}", e);
                Error::FailedToOpen("error")
            })?;

        inp.clear();
        error_file
            .read_to_string(&mut inp)
            .await
            .map_err(|e| {
                tracing::error!("Failed to read error status: {}", e);
                Error::FailedToRead("error")
            })?;

        let temp = inp.trim();
        match temp {
            "none" | "" => {}
            // Skipped since firmware is the same
            "preparing:firmware-invalid" => {
                tracing::info!("Firmware already up to date, skipping flash");
                return Ok(());
            }
            _ => {
                let resp: Vec<&str> = temp.split(':').collect();
                assert_eq!(resp.len(), 2);

                tracing::error!(
                    "Flashing error at stage '{}': code '{}'",
                    resp[0],
                    resp[1]
                );

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
