//! Library to provide flashing capabilities for [CC1352P7] in [BeagleConnect Freedom]. This is the
//! main processor, and can be flashed just by connecting over USB.
//!
//! BSL command details can be found in [Technical Specification].
//!
//! [BeagleConnect Freedom]: https://www.beagleboard.org/boards/beagleconnect-freedom
//! [CC1352P7]: https://www.ti.com/product/CC1352P7
//! [Technical Specification]: https://www.ti.com/lit/ug/swcu192/swcu192.pdf?ts=1741089110661&ref_url=https%253A%252F%252Fwww.ti.com%252Fproduct%252FCC1352P7

use std::{io, time::Duration};

use futures::channel::mpsc;
use serialport::SerialPort;
use thiserror::Error;
use tracing::{error, info, warn};

use crate::{
    Status,
    helpers::{chan_send, parse_bin},
};

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

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Error, Debug)]
/// Errors for CC1352P7
pub enum Error {
    /// Status for failing flash erase or program operation
    #[error("Status for failing flash erase or program operation")]
    FlashFail,
    /// Bootloader sent unexpected response
    #[error("Bootloader sent unexpected response")]
    UnknownResponse,
    /// Bootloader Responded with Nack
    #[error("Bootloader Responded with Nack")]
    Nack,
    /// Failed to start Bootloader
    #[error("Failed to start Bootloader")]
    FailedToStartBootloader,
    /// Flashed image is not valid
    #[error("Flashed image is not valid")]
    InvalidImage,
    /// Failed to open serial port
    #[error("Failed to open serial port")]
    FailedToOpenPort,
    /// Aborted before completing
    #[error("Aborted before completing")]
    Aborted,
    #[error("IO Error: {0}")]
    IoError(io::Error),
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::IoError(value)
    }
}

struct BeagleConnectFreedom<S: SerialPort> {
    port: S,
}

impl<S> BeagleConnectFreedom<S>
where
    S: SerialPort,
{
    fn new(port: S) -> Result<Self> {
        let mut bcf = BeagleConnectFreedom { port };

        bcf.invoke_bootloader()?;
        bcf.send_sync()?;

        Ok(bcf)
    }

    fn wait_for_ack(&mut self) -> Result<()> {
        let mut buf = [0u8; 1];

        while buf[0] == 0x00 {
            self.port.read_exact(&mut buf)?;
        }

        match buf[0] {
            ACK => Ok(()),
            NACK => Err(Error::Nack),
            _ => Err(Error::UnknownResponse),
        }
    }

    fn invoke_bootloader(&mut self) -> Result<()> {
        info!("Invoke Bootloader");

        let _ = self
            .port
            .set_break()
            .map_err(|_| Error::FailedToStartBootloader)?;
        let _ = std::thread::sleep(Duration::from_secs(2));
        let _ = self
            .port
            .clear_break()
            .map_err(|_| Error::FailedToStartBootloader)?;

        let _ = std::thread::sleep(Duration::from_millis(500));
        Ok(())
    }

    fn send_sync(&mut self) -> Result<()> {
        info!("Send Sync");
        const PKT: &[u8] = &[0x55, 0x55];

        self.port.write_all(PKT)?;

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

        self.port.write_all(&[15, checksum, COMMAND_CRC32])?;
        self.port.write_all(&addr)?;
        self.port.write_all(&size)?;
        self.port.write_all(&read_repeat)?;

        self.wait_for_ack()?;

        self.port.read_exact(&mut cmd)?;
        assert_eq!(cmd[0], 6);

        let checksum = cmd[1];

        self.port.read_exact(&mut cmd_data)?;
        assert_eq!(
            checksum,
            cmd_data.iter().fold(0u8, |acc, x| acc.wrapping_add(*x))
        );

        self.send_ack()?;

        Ok(u32::from_be_bytes(cmd_data))
    }

    fn send_ack(&mut self) -> Result<(), Error> {
        const PKT: &[u8] = &[0x00, ACK];
        self.port.write_all(PKT).map_err(Into::into)
    }

    fn send_bank_erase(&mut self) -> Result<(), Error> {
        const CMD: &[u8] = &[3, COMMAND_BANK_ERASE, COMMAND_BANK_ERASE];

        self.port.write_all(CMD)?;

        self.wait_for_ack()?;
        self.get_status()
    }

    fn get_status(&mut self) -> Result<(), Error> {
        const CMD: &[u8] = &[3, COMMAND_GET_STATUS, COMMAND_GET_STATUS];
        let mut resp = [0u8; 1];

        self.port.write_all(CMD)?;

        self.wait_for_ack()?;

        while resp[0] == 0x00 {
            self.port.read(&mut resp)?;
        }

        self.port.read(&mut resp)?;
        self.port.read(&mut resp)?;

        self.send_ack()?;

        match resp[0] {
            0x40 => Ok(()),
            0x41 => panic!("Unknown Command"),
            0x42 => panic!("Invalid Command"),
            0x43 => panic!("Invalid Address"),
            0x44 => Err(Error::FlashFail),
            _ => Err(Error::UnknownResponse),
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

        self.port.write_all(&[11, checksum, COMMAND_DOWNLOAD])?;
        self.port.write_all(&addr)?;
        self.port.write_all(&size)?;

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
            .write_all(&[(bytes_to_write + 3) as u8, checksum, COMMAND_SEND_DATA])?;
        self.port.write_all(&data[..bytes_to_write])?;

        self.wait_for_ack()?;
        self.get_status()?;

        Ok(bytes_to_write)
    }

    fn send_reset(&mut self) -> Result<(), Error> {
        const CMD: &[u8] = &[3, COMMAND_RESET, COMMAND_RESET];

        self.port.write_all(CMD)?;
        self.wait_for_ack()
    }

    fn verify(&mut self, crc32: u32) -> Result<bool> {
        self.crc32().map(|x| x == crc32)
    }
}

impl<S> Drop for BeagleConnectFreedom<S>
where
    S: SerialPort,
{
    fn drop(&mut self) {
        let _ = self.send_reset();
    }
}

const fn progress(off: usize) -> f32 {
    (off as f32) / (FIRMWARE_SIZE as f32)
}

fn check_arc(cancel: Option<&std::sync::Weak<()>>) -> Result<()> {
    match cancel {
        Some(x) if x.strong_count() == 0 => Err(Error::Aborted),
        _ => Ok(()),
    }
}

/// Flash BeagleConnect Freedom. Also provides optional progress and abort mechanism.
///
/// # Firmware
///
/// Firmware type is auto detected. Supported firmwares:
///
/// - Raw binary
/// - Ti-TXT
/// - iHex
///
/// # Aborting
///
/// The process can be aborted by dropping all strong references to the [`Arc`] that owns the
/// [`Weak`] passed as `cancel`.
///
/// [`Arc`]: std::sync::Arc
/// [`Weak`]: std::sync::Weak
pub fn flash(
    firmware: &[u8],
    port: &str,
    verify: bool,
    mut chan: Option<mpsc::Sender<Status>>,
    cancel: Option<std::sync::Weak<()>>,
) -> Result<()> {
    let firmware_bin = parse_bin(firmware).map_err(|_| Error::InvalidImage)?;

    chan_send(chan.as_mut(), Status::Preparing);

    let port = serialport::new(port, 115200)
        .timeout(Duration::from_millis(500))
        .open_native()
        .map_err(|_| Error::FailedToOpenPort)?;
    let mut bcf = BeagleConnectFreedom::new(port)?;
    info!("BeagleConnectFreedom Connected");

    let _ = check_arc(cancel.as_ref())?;
    chan_send(chan.as_mut(), Status::Flashing(0.0));

    let img_crc32 = crc32fast::hash(
        &firmware_bin
            .to_bytes(0..(FIRMWARE_SIZE as usize), Some(0xff))
            .unwrap(),
    );
    if bcf.verify(img_crc32)? {
        warn!("Skipping flashing same image");
        return Ok(());
    }

    let _ = check_arc(cancel.as_ref())?;
    info!("Erase Flash");
    bcf.send_bank_erase()?;

    info!("Start Flashing");

    let _ = check_arc(cancel.as_ref())?;
    for (start_address, data) in firmware_bin.segments_list() {
        let mut offset = 0;
        assert!(data.len() % 2 == 0);

        bcf.send_download(
            start_address.try_into().unwrap(),
            data.len().try_into().unwrap(),
        )?;
        while offset < data.len() {
            offset += bcf.send_data(&data[offset..])?;

            chan_send(
                chan.as_mut(),
                Status::Flashing(progress(start_address + offset)),
            );
            let _ = check_arc(cancel.as_ref())?;
        }
    }

    let res = if verify {
        chan_send(chan.as_mut(), Status::Verifying);
        if bcf.verify(img_crc32)? {
            info!("Flashing Successful");
            Ok(())
        } else {
            error!("Invalid CRC32 in Flash. The flashed image might be corrupted");
            Err(Error::InvalidImage.into())
        }
    } else {
        Ok(())
    };

    res
}

/// Returns all paths to ports having BeagleConnect Freedom.
pub fn ports() -> std::collections::HashSet<String> {
    serialport::available_ports()
        .expect("Unsupported OS")
        .into_iter()
        .filter(|x| {
            if cfg!(target_os = "linux") {
                match &x.port_type {
                    serialport::SerialPortType::UsbPort(y) => {
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
        .collect()
}
