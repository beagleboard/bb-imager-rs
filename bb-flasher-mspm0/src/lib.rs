//! A library to flash the MSPM0 co-processor using its ROM bootloader (BSL).
//!
//! Supported interfaces are gated behind feature flags:
//!
//! - `uart` (default): `uart::flash` over a serial port. Cross-platform.
//! - `i2c`: `i2c::flash` over an I2C bus. Linux only.
//!
//! Firmware can be provided as a raw binary or as text in any format supported by [`bin_file`]
//! (Intel HEX, Motorola S-Record, TI-TXT).

use thiserror::Error;

mod bsl;
mod helpers;
#[cfg(all(feature = "i2c", target_os = "linux"))]
pub mod i2c;
#[cfg(test)]
pub(crate) mod mock_bsl;
#[cfg(feature = "uart")]
pub mod uart;

/// Result type for this crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Error, Debug)]
#[non_exhaustive]
/// Errors for MSPM0
pub enum Error {
    /// Failed to connect to the bootloader (BSL).
    #[error("Failed to connect to the bootloader (BSL). Please put the board in BSL mode.")]
    ConnectionFail,
    /// Aborted before completing
    #[error("Aborted before completing.")]
    Aborted,
    /// BSL reported an incorrect packet header.
    #[error("Header is incorrect")]
    HeaderIncorrect,
    /// BSL reported an incorrect packet checksum.
    #[error("Checksum is incorrect")]
    ChecksumIncorrect,
    /// BSL reported a packet of size 0.
    #[error("Invalid packet size of 0")]
    PktSizeZero,
    /// BSL reported a packet exceeding its buffer size.
    #[error("Packet size is too big")]
    PktSize2Big,
    /// BSL reported an unknown error.
    #[error("Unknown error occurred")]
    Unknown,
    /// BSL does not support the requested baud rate.
    #[error("Unknown baud rate")]
    UnknownBaudRate,
    /// Unknown error occurred during IO.
    #[error("Unknown Error during IO. Please check logs for more information.")]
    IoError {
        /// Underlying I/O error.
        #[from]
        #[source]
        source: std::io::Error,
    },
    /// BSL sent a response that could not be parsed.
    #[error("MSPM0 BSL sent an unknown message. Please check logs for more information.")]
    InvalidResponse,
    /// Flashed image is not valid
    #[error("Flashed image is not valid.")]
    InvalidImage,
    /// Failed to open serial port
    #[error("Failed to open serial port.")]
    FailedToOpenPort,
    /// Failed to set a GPIO line.
    #[error("Failed to set GPIO")]
    #[cfg(target_os = "linux")]
    GpioIoError {
        /// Underlying GPIO error.
        #[from]
        #[source]
        source: gpiocdev::Error,
    },
    /// Failed to find a GPIO line with the given name.
    #[error("Failed to open {0}")]
    #[cfg(target_os = "linux")]
    GpioOpenError(String),

    /// Failed to change the BSL baud rate.
    #[error("Failed to update baud rate")]
    ChangeBaudRate,
}

/// Flashing status
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Status {
    /// Preparing the device for flashing.
    Preparing,
    /// Firmware is being written. Progress is in `0.0..=1.0`.
    Flashing(f32),
    /// Verifying the flashed firmware.
    Verifying,
}

/// Returns a `prep_hook` that puts MSPM0 into BSL mode using GPIO lines found by name.
///
/// The returned closure holds the `bsl` line high while pulsing the `reset` line, then releases
/// both lines back to inputs. Pass it as `prep_hook` to `uart::flash` or `i2c::flash`.
#[cfg(target_os = "linux")]
pub fn bsl_gpio_cdev_by_name(reset: String, bsl: String) -> impl FnOnce() -> Result<()> {
    use gpiocdev::line::Value;
    use std::time::Duration;

    fn open_gpio(name: String) -> crate::Result<gpiocdev::Request> {
        let pin =
            gpiocdev::find_named_line(&name).ok_or(crate::Error::GpioOpenError(name.clone()))?;
        tracing::info!("Found Pin {name}: {:#?}", pin);
        gpiocdev::Request::builder()
            .with_found_line(&pin)
            .as_output(Value::Inactive)
            .request()
            .map_err(Into::into)
    }

    move || {
        let reset = open_gpio(reset)?;
        let bsl = open_gpio(bsl)?;

        tracing::info!("Starting BSL");

        bsl.set_lone_value(gpiocdev::line::Value::Active)?;
        std::thread::sleep(Duration::from_secs(1));

        reset.set_lone_value(gpiocdev::line::Value::Active)?;
        std::thread::sleep(Duration::from_secs(1));

        reset.set_lone_value(gpiocdev::line::Value::Inactive)?;
        reset.reconfigure(reset.config().as_input())?;

        std::thread::sleep(Duration::from_secs(1));

        bsl.set_lone_value(gpiocdev::line::Value::Inactive)?;
        bsl.reconfigure(bsl.config().as_input())?;

        Ok(())
    }
}
