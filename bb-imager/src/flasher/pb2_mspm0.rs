use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
use thiserror::Error;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::DownloadFlashingStatus;

const PATH: &str = "/sys/class/firmware/mspm0l1105/";
const EEPROM: &str = "/sys/bus/i2c/devices/0-0050/eeprom";
const FIRMWARE_SIZE: usize = 32 * 1024;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to open EEPROM")]
    FailedToOpenEeprom,
    #[error("Failed to open status for firmware upload")]
    FailedToOpenStatus,
    #[error("Failed to flash at {stage} due to {code}")]
    FlashingError { stage: String, code: String },
}

pub async fn flash(
    img: bin_file::BinFile,
    chan: &tokio::sync::mpsc::Sender<crate::DownloadFlashingStatus>,
    config: FlashingPb2Mspm0Config,
) -> crate::error::Result<()> {
    let dst = Path::new(PATH);
    let firmware = img.to_bytes(0..FIRMWARE_SIZE, None).unwrap();

    let mut eeprom_contents = Vec::new();

    // Copy the current EEPROM contents
    if config.eeprom {
        let mut eeprom = File::open(EEPROM)
            .await
            .map_err(|_| Error::FailedToOpenEeprom)?;
        eeprom.read_to_end(&mut eeprom_contents).await?;
    }

    flash_fw_api(dst, &firmware, chan).await?;

    // Write back EEPROM contents
    if config.eeprom {
        let mut file = sysfs_w_open(Path::new(EEPROM))
            .await
            .map_err(|_| Error::FailedToOpenEeprom)?;
        file.write_all(&eeprom_contents).await.unwrap();
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

async fn flash_fw_api(
    base: &Path,
    firmware: &[u8],
    chan: &tokio::sync::mpsc::Sender<crate::DownloadFlashingStatus>,
) -> crate::error::Result<()> {
    let loading_path = base.join("loading");
    let data_path = base.join("data");
    let status_path = base.join("status");
    let error_path = base.join("error");
    let remaining_size_path = base.join("remaining_size");

    let mut inp = String::new();

    // Initial firmware upload
    {
        let mut loading_file = sysfs_w_open(&loading_path).await.unwrap();
        loading_file.write_all(b"1").await.unwrap();
        loading_file.flush().await.unwrap();

        let mut data_file = sysfs_w_open(&data_path).await.unwrap();
        data_file.write_all(firmware).await.unwrap();

        loading_file.write_all(b"0").await.unwrap();
    }

    // Wait for flashing to finish
    loop {
        // sysfs entries cause weird stuff if kept open after a single read/write
        let mut status_file = File::open(&status_path)
            .await
            .map_err(|_| Error::FailedToOpenStatus)?;

        inp.clear();
        status_file.read_to_string(&mut inp).await.unwrap();

        match inp.trim() {
            "idle" => break,
            "preparing" => {
                let _ = chan.try_send(DownloadFlashingStatus::Preparing);
            }
            "transferring" => {
                let mut prog = String::with_capacity(3);
                let mut size_file = File::open(&remaining_size_path)
                    .await
                    .map_err(|_| Error::FailedToOpenStatus)?;
                size_file.read_to_string(&mut prog).await.unwrap();

                let p = prog.trim().parse::<usize>().unwrap();
                let _ = chan.try_send(DownloadFlashingStatus::FlashingProgress(
                    (firmware.len() - p) as f32 / firmware.len() as f32,
                ));
            }
            "programming" => {
                let _ = chan.try_send(DownloadFlashingStatus::Verifying);
            }
            _ => {}
        }
    }

    // Check for error
    {
        let mut error_file = File::open(&error_path)
            .await
            .map_err(|_| Error::FailedToOpenStatus)?;

        inp.clear();
        error_file.read_to_string(&mut inp).await.unwrap();

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

pub fn possible_devices() -> std::collections::HashSet<crate::Destination> {
    HashSet::from([crate::Destination::file(PathBuf::from(PATH))])
}

#[derive(Clone, Debug)]
pub struct FlashingPb2Mspm0Config {
    pub eeprom: bool,
}

impl Default for FlashingPb2Mspm0Config {
    fn default() -> Self {
        Self { eeprom: true }
    }
}
