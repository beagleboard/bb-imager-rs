//! Helpers to enable flashing BeagleConnect Freedom firmware

use std::{
    io::{self, Read},
    time::Duration,
};

use crate::{error::Result, util};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_serial::SerialPort;
use tracing::{error, info, warn};

const ACK: u8 = 0xcc;
const NACK: u8 = 0x33;

const COMMAND_DOWNLOAD: u8 = 0x21;
const COMMAND_GET_STATUS: u8 = 0x23;
const COMMAND_SEND_DATA: u8 = 0x24;
const COMMAND_RESET: u8 = 0x25;
const COMMAND_CRC32: u8 = 0x27;
const COMMAND_BANK_ERASE: u8 = 0x2c;

const COMMAND_MAX_SIZE: u8 = u8::MAX - 3;

const FIRMWARE_SIZE: u32 = 704 * 1024;

#[derive(Error, Debug, Clone, Copy)]
pub enum Error {
    #[error("Status for unknown Command")]
    UnknownCmd,
    #[error("Status for invalid Command (in other words, incorrect packet size)")]
    InvalidCmd,
    #[error("Status for invalid input address")]
    InvalidAddr,
    #[error("Status for failing flash erase or program operation")]
    FlashFail,
    #[error("Bootloader sent unexpected response")]
    UnknownResponse,
    #[error("Failed to send Command to Bootloader")]
    FailedToSend,
    #[error("Bootloader failed to respond")]
    FailedToRead,
    #[error("Bootloader Responded with Nack")]
    Nack,
    #[error("Failed to start Bootloader")]
    FailedToStartBootloader,
    #[error("Failed to open firmware image")]
    FailedToOpenImage,
    #[error("Flashed image is not valid")]
    InvalidImage,
}

impl From<u8> for Error {
    fn from(value: u8) -> Self {
        match value {
            0x41 => Self::UnknownCmd,
            0x42 => Self::InvalidCmd,
            0x43 => Self::InvalidAddr,
            0x44 => Self::FlashFail,
            _ => Self::UnknownResponse,
        }
    }
}

struct BeagleConnectFreedom {
    port: tokio_serial::SerialStream,
}

impl BeagleConnectFreedom {
    async fn new(port: tokio_serial::SerialStream) -> Result<Self> {
        let mut bcf = BeagleConnectFreedom { port };

        bcf.invoke_bootloader()
            .await
            .map_err(|_| Error::FailedToStartBootloader)?;

        bcf.send_sync().await?;

        Ok(bcf)
    }

    async fn wait_for_ack(&mut self) -> Result<(), Error> {
        let mut buf = [0u8; 1];

        while buf[0] == 0x00 {
            AsyncReadExt::read_exact(&mut self.port, &mut buf)
                .await
                .map_err(|_| Error::FailedToRead)?;
        }

        match buf[0] {
            ACK => Ok(()),
            NACK => Err(Error::Nack),
            _ => Err(Error::UnknownResponse),
        }
    }

    async fn invoke_bootloader(&mut self) -> io::Result<()> {
        info!("Invoke Bootloader");

        let _ = self.port.set_break();
        let _ = tokio::time::sleep(Duration::from_secs(2)).await;
        let _ = self.port.clear_break();

        let _ = tokio::time::sleep(Duration::from_millis(500)).await;
        Ok(())
    }

    async fn send_sync(&mut self) -> Result<(), Error> {
        info!("Send Sync");
        const PKT: &[u8] = &[0x55, 0x55];

        self.port
            .write_all(PKT)
            .await
            .map_err(|_| Error::FailedToSend)?;

        self.wait_for_ack().await
    }

    async fn crc32(&mut self) -> Result<u32> {
        let addr = 0u32.to_be_bytes();
        let size = FIRMWARE_SIZE.to_be_bytes();
        let read_repeat = 0u32.to_be_bytes();
        let mut cmd = [0u8; 2];
        let mut cmd_data = [0u8; 4];

        let checksum: u8 = size
            .iter()
            .chain(&[COMMAND_CRC32])
            .fold(0u8, |acc, t| acc.wrapping_add(*t));

        self.port
            .write_all(&[15, checksum, COMMAND_CRC32])
            .await
            .map_err(|_| Error::FailedToSend)?;
        self.port
            .write_all(&addr)
            .await
            .map_err(|_| Error::FailedToSend)?;
        self.port
            .write_all(&size)
            .await
            .map_err(|_| Error::FailedToSend)?;
        self.port
            .write_all(&read_repeat)
            .await
            .map_err(|_| Error::FailedToSend)?;

        self.wait_for_ack().await?;

        AsyncReadExt::read_exact(&mut self.port, &mut cmd)
            .await
            .map_err(|_| Error::FailedToRead)?;
        assert_eq!(cmd[0], 6);

        let checksum = cmd[1];

        AsyncReadExt::read_exact(&mut self.port, &mut cmd_data)
            .await
            .map_err(|_| Error::FailedToRead)?;
        assert_eq!(
            checksum,
            cmd_data.iter().fold(0u8, |acc, x| acc.wrapping_add(*x))
        );

        self.send_ack().await?;

        Ok(u32::from_be_bytes(cmd_data))
    }

    async fn send_ack(&mut self) -> Result<(), Error> {
        const PKT: &[u8] = &[0x00, ACK];
        self.port
            .write_all(PKT)
            .await
            .map_err(|_| Error::FailedToSend)
    }

    async fn send_bank_erase(&mut self) -> Result<(), Error> {
        const CMD: &[u8] = &[3, COMMAND_BANK_ERASE, COMMAND_BANK_ERASE];

        self.port
            .write_all(CMD)
            .await
            .map_err(|_| Error::FailedToSend)?;

        self.wait_for_ack().await?;
        self.get_status().await
    }

    async fn get_status(&mut self) -> Result<(), Error> {
        const CMD: &[u8] = &[3, COMMAND_GET_STATUS, COMMAND_GET_STATUS];
        let mut resp = [0u8; 1];

        self.port
            .write_all(CMD)
            .await
            .map_err(|_| Error::FailedToSend)?;

        self.wait_for_ack().await?;

        while resp[0] == 0x00 {
            AsyncReadExt::read(&mut self.port, &mut resp)
                .await
                .map_err(|_| Error::FailedToRead)?;
        }

        AsyncReadExt::read(&mut self.port, &mut resp)
            .await
            .map_err(|_| Error::FailedToRead)?;
        AsyncReadExt::read(&mut self.port, &mut resp)
            .await
            .map_err(|_| Error::FailedToRead)?;

        self.send_ack().await?;

        match resp[0] {
            0x40 => Ok(()),
            _ => Err(Error::from(resp[0])),
        }
    }

    async fn send_download(&mut self, addr: u32, size: u32) -> Result<(), Error> {
        let addr = addr.to_be_bytes();
        let size = size.to_be_bytes();

        let checksum: u8 = addr
            .into_iter()
            .chain(size)
            .chain([COMMAND_DOWNLOAD])
            .fold(0u8, |acc, t| acc.wrapping_add(t));

        self.port
            .write_all(&[11, checksum, COMMAND_DOWNLOAD])
            .await
            .map_err(|_| Error::FailedToSend)?;
        self.port
            .write_all(&addr)
            .await
            .map_err(|_| Error::FailedToSend)?;
        self.port
            .write_all(&size)
            .await
            .map_err(|_| Error::FailedToSend)?;

        self.wait_for_ack().await?;
        self.get_status().await
    }

    async fn send_data(&mut self, data: &[u8]) -> Result<usize> {
        let bytes_to_write = std::cmp::min(data.len(), usize::from(COMMAND_MAX_SIZE));

        let checksum = data[..bytes_to_write]
            .iter()
            .chain(&[COMMAND_SEND_DATA])
            .fold(0u8, |acc, t| acc.wrapping_add(*t));

        self.port
            .write_all(&[(bytes_to_write + 3) as u8, checksum, COMMAND_SEND_DATA])
            .await
            .map_err(|_| Error::FailedToSend)?;
        self.port
            .write_all(&data[..bytes_to_write])
            .await
            .map_err(|_| Error::FailedToSend)?;

        self.wait_for_ack().await?;
        self.get_status().await?;

        Ok(bytes_to_write)
    }

    async fn send_reset(&mut self) -> Result<(), Error> {
        const CMD: &[u8] = &[3, COMMAND_RESET, COMMAND_RESET];

        self.port
            .write_all(CMD)
            .await
            .map_err(|_| Error::FailedToSend)?;

        self.wait_for_ack().await
    }

    async fn verify(&mut self, crc32: u32) -> Result<bool> {
        self.crc32().await.map(|x| x == crc32)
    }
}

fn progress(off: u32) -> f32 {
    (off as f32) / (FIRMWARE_SIZE as f32)
}

fn open_firmware(mut img: crate::img::OsImage) -> Result<Vec<u8>> {
    let mut img_data = Vec::with_capacity(FIRMWARE_SIZE as usize);
    img.read_to_end(&mut img_data)
        .map_err(|_| Error::InvalidImage)?;

    match String::from_utf8(img_data) {
        Ok(x) => util::bin_file_from_str(x)
            .map_err(|_| Error::InvalidImage)?
            .to_bytes(0..(FIRMWARE_SIZE as usize), Some(0xFF))
            .map_err(|_| Error::InvalidImage.into())
            .map(|x| x.to_vec()),
        Err(e) => {
            let img_data = e.into_bytes();
            if img_data.len() != FIRMWARE_SIZE as usize {
                Err(Error::InvalidImage.into())
            } else {
                Ok(img_data)
            }
        }
    }
}

pub async fn flash(
    img: crate::img::OsImage,
    port: tokio_serial::SerialStream,
    chan: &tokio::sync::mpsc::Sender<crate::DownloadFlashingStatus>,
    verify: bool,
) -> Result<()> {
    let mut bcf = BeagleConnectFreedom::new(port).await?;
    info!("BeagleConnectFreedom Connected");

    let firmware = open_firmware(img)?;
    let img_crc32 = crc32fast::hash(firmware.as_slice());

    let _ = chan.try_send(crate::DownloadFlashingStatus::FlashingProgress(0.0));

    if bcf.verify(img_crc32).await? {
        warn!("Skipping flashing same image");
        return Ok(());
    }

    info!("Erase Flash");
    bcf.send_bank_erase().await?;

    info!("Start Flashing");

    let mut image_offset = 0;
    let mut reset_download = true;

    while image_offset < FIRMWARE_SIZE {
        while firmware[image_offset as usize] == 0xff {
            image_offset += 1;
            reset_download = true;
        }

        if reset_download {
            bcf.send_download(image_offset, FIRMWARE_SIZE - image_offset)
                .await?;
            reset_download = false;
        }

        image_offset += bcf.send_data(&firmware[(image_offset as usize)..]).await? as u32;
        let _ = chan.try_send(crate::DownloadFlashingStatus::FlashingProgress(progress(
            image_offset,
        )));
    }

    let res = if verify {
        let _ = chan.try_send(crate::DownloadFlashingStatus::Verifying);
        if bcf.verify(img_crc32).await? {
            info!("Flashing Successful");
            Ok(())
        } else {
            error!("Invalid CRC32 in Flash. The flashed image might be corrupted");
            Err(Error::InvalidImage.into())
        }
    } else {
        Ok(())
    };

    let _ = bcf.send_reset().await;

    res
}

pub fn possible_devices() -> std::collections::HashSet<crate::Destination> {
    tokio_serial::available_ports()
        .expect("Unsupported OS")
        .into_iter()
        .filter(|x| {
            if cfg!(target_os = "linux") {
                match &x.port_type {
                    tokio_serial::SerialPortType::UsbPort(y) => {
                        y.manufacturer.as_deref() == Some("BeagleBoard.org")
                            && y.product.as_deref() == Some("BeagleConnect")
                    }
                    _ => false,
                }
            } else {
                true
            }
        })
        .map(|x| x.port_name)
        .map(crate::Destination::port)
        .collect()
}

#[derive(Clone, Debug)]
pub struct FlashingBcfConfig {
    pub verify: bool,
}

impl FlashingBcfConfig {
    pub fn update_verify(mut self, verify: bool) -> Self {
        self.verify = verify;
        self
    }
}

impl Default for FlashingBcfConfig {
    fn default() -> Self {
        Self { verify: true }
    }
}
