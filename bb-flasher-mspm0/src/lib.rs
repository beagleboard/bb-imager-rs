use thiserror::Error;

mod bsl;
mod helpers;
#[cfg(feature = "uart")]
pub mod uart;
#[cfg(all(feature = "i2c", target_os = "linux"))]
pub mod i2c;

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

/// Flashing status
pub enum Status {
    Preparing,
    Flashing(f32),
    Verifying,
}
