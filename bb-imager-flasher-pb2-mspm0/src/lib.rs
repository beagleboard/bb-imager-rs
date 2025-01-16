use std::path::{Path, PathBuf};
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
pub enum Error {
    #[error("Failed to open {0}")]
    FailedToOpen(&'static str),
    #[error("Failed to read {0}")]
    FailedToRead(&'static str),
    #[error("Failed to write to {0}")]
    FailedToWrite(&'static str),
    #[error("Failed to seek {0}")]
    FailedToSeek(&'static str),
    #[error("Failed to flash at {stage} due to {code}")]
    FlashingError { stage: String, code: String },
    #[error("Invalid firmware")]
    InvalidFirmware,
}

type Result<T, E = Error> = std::result::Result<T, E>;

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
            .map_err(|_| Error::FailedToOpen("EEPROM"))?;
        eeprom
            .read_to_end(&mut eeprom_contents)
            .await
            .map_err(|_| Error::FailedToRead("EEPROM"))?;
    }

    flash_fw_api(Path::new(PATH), &firmware, chan).await?;

    // Write back EEPROM contents
    if persist_eeprom {
        let mut eeprom = sysfs_w_open(Path::new(EEPROM))
            .await
            .map_err(|_| Error::FailedToOpen("EEPROM"))?;
        eeprom
            .write_all(&eeprom_contents)
            .await
            .map_err(|_| Error::FailedToWrite("EEPROM"))?;
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
        .map_err(|_| Error::FailedToOpen(name))?;

    if temp {
        Ok(())
    } else {
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
        let mut loading_file = sysfs_w_open(&loading_path)
            .await
            .map_err(|_| Error::FailedToOpen("loading"))?;
        loading_file
            .write_all(b"1")
            .await
            .map_err(|_| Error::FailedToWrite("loading"))?;
        loading_file
            .flush()
            .await
            .map_err(|_| Error::FailedToWrite("loading"))?;

        let mut data_file = sysfs_w_open(&data_path)
            .await
            .map_err(|_| Error::FailedToOpen("data"))?;
        data_file
            .write_all(firmware)
            .await
            .map_err(|_| Error::FailedToWrite("data"))?;

        loading_file
            .write_all(b"0")
            .await
            .map_err(|_| Error::FailedToWrite("loading"))?;
    }

    // Wait for flashing to finish
    loop {
        // sysfs entries cause weird stuff if kept open after a single read/write
        let mut status_file = File::open(&status_path)
            .await
            .map_err(|_| Error::FailedToOpen("status"))?;

        inp.clear();
        status_file
            .read_to_string(&mut inp)
            .await
            .map_err(|_| Error::FailedToRead("status"))?;

        match inp.trim() {
            "idle" => break,
            "preparing" => {
                let _ = chan.try_send(Status::Preparing);
            }
            "transferring" => {
                let mut prog = String::with_capacity(3);
                let mut size_file = File::open(&remaining_size_path)
                    .await
                    .map_err(|_| Error::FailedToOpen("remaining_size"))?;
                size_file
                    .read_to_string(&mut prog)
                    .await
                    .map_err(|_| Error::FailedToRead("remaining_size"))?;

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
        let mut error_file = File::open(&error_path)
            .await
            .map_err(|_| Error::FailedToOpen("error"))?;

        inp.clear();
        error_file
            .read_to_string(&mut inp)
            .await
            .map_err(|_| Error::FailedToRead("error"))?;

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
                }
                .into());
            }
        }
    }

    Ok(())
}

pub fn device() -> Device {
    Device {
        name: DEVICE.to_string(),
        path: PathBuf::from(PATH),
        flash_size: FIRMWARE_SIZE,
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Status {
    Preparing,
    Flashing(f32),
    Verifying,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "zvariant", derive(zvariant::Type))]
pub struct Device {
    pub name: String,
    pub path: PathBuf,
    pub flash_size: usize,
}
