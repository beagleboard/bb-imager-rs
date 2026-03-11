use thiserror::Error;
use tokio::sync::mpsc;

use std::time::Duration;

use crate::{
    bsl::Mspm0,
    helpers::{chan_send, check_token, parse_bin},
};

mod bsl;
mod helpers;

const BSL_UART_BAUD_RATE: u32 = 9600;
const BSL_UART_DATA_BITS: serialport::DataBits = serialport::DataBits::Eight;
const BSL_UART_STOP_BITS: serialport::StopBits = serialport::StopBits::One;
const BSL_UART_PARITY: serialport::Parity = serialport::Parity::None;

/// Flashing status
pub enum Status {
    Preparing,
    Flashing(f32),
    Verifying,
}

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Error, Debug)]
/// Errors for MSPM0
pub enum Error {
    /// Aborted before completing
    #[error("Aborted before completing.")]
    Aborted,
    #[error("Header is incorrect")]
    HeaderIncorrect,
    #[error("Checksum is incorrect")]
    ChecksumIncorrect,
    #[error("Invalid packet size of 0")]
    PktSizeZero,
    #[error("Packet size is too big")]
    PktSize2Big,
    #[error("Unknown error occured")]
    Unknown,
    #[error("Unknown baud rate")]
    UnknownBaudRate,
    /// Unknown error occured during IO.
    #[error("Unknown Error during IO. Please check logs for more information.")]
    IoError {
        #[from]
        #[source]
        source: std::io::Error,
    },
    #[error("MSPM0 BSL sent an unknown message. Please check logs for more information.")]
    InvalidResponse,
    /// Flashed image is not valid
    #[error("Flashed image is not valid.")]
    InvalidImage,
    /// Failed to open serial port
    #[error("Failed to open serial port.")]
    FailedToOpenPort,
}

pub fn flash(
    firmware: &[u8],
    fw_size: u32,
    port: &str,
    verify: bool,
    mut chan: Option<mpsc::Sender<Status>>,
    cancel: Option<tokio_util::sync::CancellationToken>,
) -> Result<()> {
    let firmware = parse_bin(firmware, fw_size as usize).map_err(|_| Error::InvalidImage)?;

    chan_send(chan.as_mut(), Status::Preparing);

    let port = serialport::new(port, BSL_UART_BAUD_RATE)
        .parity(BSL_UART_PARITY)
        .stop_bits(BSL_UART_STOP_BITS)
        .data_bits(BSL_UART_DATA_BITS)
        .timeout(Duration::from_millis(2000))
        .open_native()
        .map_err(|_| Error::FailedToOpenPort)?;
    let mut mspm0 = Mspm0::new(port)?;
    tracing::info!("MSPM0 Connected");

    mspm0.unlock()?;

    let img_crc32 = crc32fast::hash(&firmware);

    check_token(cancel.as_ref())?;
    chan_send(chan.as_mut(), Status::Flashing(0.0));

    let cur_crc = mspm0.standalone_verification(fw_size)?;
    if cur_crc == img_crc32 {
        tracing::warn!("Skipping flashing same image");
        return mspm0.start_application();
    }

    check_token(cancel.as_ref())?;
    mspm0.mass_erase()?;

    tracing::info!("Start Flashing");

    check_token(cancel.as_ref())?;
    let mut address = 0;
    while address < firmware.len() {
        address += mspm0.program_data(address.try_into().unwrap(), &firmware[address..])?;
        check_token(cancel.as_ref())?;
    }

    if verify {
        let cur_crc = mspm0.standalone_verification(fw_size)?;
        if cur_crc != img_crc32 {
            tracing::error!("Invalid CRC32 in Flash. The flashed image might be corrupted");
            return Err(Error::InvalidImage);
        }
    }

    mspm0.start_application()
}

/// Returns all paths to serial ports.
pub fn ports() -> std::collections::HashSet<String> {
    serialport::available_ports()
        .expect("Unsupported OS")
        .into_iter()
        .map(|x| x.port_name)
        .collect()
}
