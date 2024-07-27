//! Helpers to enable flashing BeagleConnect Freedom firmware

use std::{
    io::{self, Read},
    thread::sleep,
    time::Duration,
};

use crate::error::Result;
use crate::FlashingStatus;
use futures_util::Stream;
use thiserror::Error;
use tokio::io::AsyncReadExt;
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
    #[error("Failed to open supplied Port")]
    FailedToOpenPort,
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
    port: Box<dyn serialport::SerialPort>,
}

impl BeagleConnectFreedom {
    fn new(port: &str) -> Result<Self> {
        let port = serialport::new(port, 500000)
            .timeout(Duration::from_millis(500))
            .open()
            .map_err(|_| Error::FailedToOpenPort)?;

        let mut bcf = BeagleConnectFreedom { port };

        bcf.invoke_bootloader()
            .map_err(|_| Error::FailedToStartBootloader)?;
        bcf.send_sync()?;

        Ok(bcf)
    }

    fn wait_for_ack(&mut self) -> Result<(), Error> {
        let mut buf = [0u8; 1];

        while buf[0] == 0x00 {
            self.port
                .read_exact(&mut buf)
                .map_err(|_| Error::FailedToRead)?;
        }

        match buf[0] {
            ACK => Ok(()),
            NACK => Err(Error::Nack),
            _ => Err(Error::UnknownResponse),
        }
    }

    fn invoke_bootloader(&mut self) -> io::Result<()> {
        let mut buf = [0u8; 100];

        let _ = self.port.set_break();
        sleep(Duration::from_millis(250));
        let _ = self.port.clear_break();

        let _ = self.port.read(&mut buf);

        sleep(Duration::from_millis(100));
        Ok(())
    }

    fn send_sync(&mut self) -> Result<(), Error> {
        const PKT: &[u8] = &[0x55, 0x55];

        self.port.write_all(PKT).map_err(|_| Error::FailedToSend)?;

        self.wait_for_ack()
    }

    fn crc32(&mut self) -> Result<u32> {
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
            .map_err(|_| Error::FailedToSend)?;
        self.port
            .write_all(&addr)
            .map_err(|_| Error::FailedToSend)?;
        self.port
            .write_all(&size)
            .map_err(|_| Error::FailedToSend)?;
        self.port
            .write_all(&read_repeat)
            .map_err(|_| Error::FailedToSend)?;

        self.wait_for_ack()?;

        self.port
            .read_exact(&mut cmd)
            .map_err(|_| Error::FailedToRead)?;
        assert_eq!(cmd[0], 6);

        let checksum = cmd[1];

        self.port
            .read_exact(&mut cmd_data)
            .map_err(|_| Error::FailedToRead)?;
        assert_eq!(
            checksum,
            cmd_data.iter().fold(0u8, |acc, x| acc.wrapping_add(*x))
        );

        self.send_ack()?;

        Ok(u32::from_be_bytes(cmd_data))
    }

    fn send_ack(&mut self) -> Result<(), Error> {
        const PKT: &[u8] = &[0x00, ACK];
        self.port.write_all(PKT).map_err(|_| Error::FailedToSend)
    }

    fn send_bank_erase(&mut self) -> Result<(), Error> {
        const CMD: &[u8] = &[3, COMMAND_BANK_ERASE, COMMAND_BANK_ERASE];

        self.port.write_all(CMD).map_err(|_| Error::FailedToSend)?;

        self.wait_for_ack()?;
        self.get_status()
    }

    fn get_status(&mut self) -> Result<(), Error> {
        const CMD: &[u8] = &[3, COMMAND_GET_STATUS, COMMAND_GET_STATUS];
        let mut resp = [0u8; 1];

        self.port.write_all(CMD).map_err(|_| Error::FailedToSend)?;

        self.wait_for_ack()?;

        while resp[0] == 0x00 {
            self.port.read(&mut resp).map_err(|_| Error::FailedToRead)?;
        }

        self.port.read(&mut resp).map_err(|_| Error::FailedToRead)?;
        self.port.read(&mut resp).map_err(|_| Error::FailedToRead)?;

        self.send_ack()?;

        match resp[0] {
            0x40 => Ok(()),
            _ => Err(Error::from(resp[0])),
        }
    }

    fn send_download(&mut self, addr: u32, size: u32) -> Result<(), Error> {
        let addr = addr.to_be_bytes();
        let size = size.to_be_bytes();

        let checksum: u8 = addr
            .into_iter()
            .chain(size)
            .chain([COMMAND_DOWNLOAD])
            .fold(0u8, |acc, t| acc.wrapping_add(t));

        self.port
            .write_all(&[11, checksum, COMMAND_DOWNLOAD])
            .map_err(|_| Error::FailedToSend)?;
        self.port
            .write_all(&addr)
            .map_err(|_| Error::FailedToSend)?;
        self.port
            .write_all(&size)
            .map_err(|_| Error::FailedToSend)?;

        self.wait_for_ack()?;
        self.get_status()
    }

    fn send_data(&mut self, data: &[u8]) -> Result<usize> {
        let bytes_to_write = std::cmp::min(data.len(), usize::from(COMMAND_MAX_SIZE));

        let checksum = data[..bytes_to_write]
            .iter()
            .chain(&[COMMAND_SEND_DATA])
            .fold(0u8, |acc, t| acc.wrapping_add(*t));

        self.port
            .write_all(&[(bytes_to_write + 3) as u8, checksum, COMMAND_SEND_DATA])
            .map_err(|_| Error::FailedToSend)?;
        self.port
            .write_all(&data[..bytes_to_write])
            .map_err(|_| Error::FailedToSend)?;

        self.wait_for_ack()?;
        self.get_status()?;

        Ok(bytes_to_write)
    }

    fn send_reset(&mut self) -> Result<(), Error> {
        const CMD: &[u8] = &[3, COMMAND_RESET, COMMAND_RESET];

        self.port.write_all(CMD).map_err(|_| Error::FailedToSend)?;

        self.wait_for_ack()
    }

    fn verify(&mut self, crc32: u32) -> Result<bool> {
        self.crc32().map(|x| x == crc32)
    }
}

impl Drop for BeagleConnectFreedom {
    fn drop(&mut self) {
        let _ = self.send_reset();
    }
}

fn progress(off: u32) -> f32 {
    (off as f32) / (FIRMWARE_SIZE as f32)
}

pub fn flash(
    mut img: crate::img::OsImage,
    port: String,
) -> impl Stream<Item = Result<crate::FlashingStatus>> {
    async_stream::try_stream! {
        yield FlashingStatus::Preparing;

        let mut firmware = Vec::with_capacity(FIRMWARE_SIZE as usize);
        img.read_to_end(&mut firmware).await?;
        drop(img);

        assert_eq!(firmware.len(), FIRMWARE_SIZE as usize);
        let img_crc32 = crc32fast::hash(firmware.as_slice());

        let mut bcf = BeagleConnectFreedom::new(&port)?;
        info!("BeagleConnectFreedom Connected");

        yield FlashingStatus::FlashingProgress(0.0);

        if bcf.verify(img_crc32)? {
            warn!("Skipping flashing same image");
            yield FlashingStatus::Finished;
            return;
        }

        info!("Erase Flash");
        bcf.send_bank_erase()?;

        info!("Start Flashing");

        let mut image_offset = 0;
        let mut reset_download = true;

        while image_offset < FIRMWARE_SIZE {
            while firmware[image_offset as usize] == 0xff {
                image_offset += 1;
                reset_download = true;
            }

            if reset_download {
                bcf.send_download(image_offset as u32, (FIRMWARE_SIZE - image_offset) as u32)?;
                reset_download = false;
            }

            image_offset += bcf.send_data(&firmware[(image_offset as usize)..])? as u32;
            yield FlashingStatus::FlashingProgress(progress(image_offset));
        }

        yield FlashingStatus::Verifying;
        if bcf.verify(img_crc32)? {
            info!("Flashing Successful");
            yield FlashingStatus::Finished;
        } else {
            error!("Invalid CRC32 in Flash. The flashed image might be corrupted");
            Err(Error::InvalidImage)?;
        }
    }
}

pub fn possible_devices() -> Result<Vec<String>> {
    Ok(serialport::available_ports()
        .map_err(|_| Error::FailedToOpenPort)?
        .into_iter()
        .map(|x| x.port_name)
        .collect())
}
