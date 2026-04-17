use thiserror::Error;

mod bsl;
mod helpers;
#[cfg(all(feature = "i2c", target_os = "linux"))]
pub mod i2c;
#[cfg(feature = "uart")]
pub mod uart;

pub type Result<T, E = Error> = std::result::Result<T, E>;

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
    #[error("Failed to set GPIO")]
    #[cfg(target_os = "linux")]
    GpioIoError {
        #[from]
        #[source]
        source: gpiocdev::Error,
    },
    #[error("Failed to open {0}")]
    #[cfg(target_os = "linux")]
    GpioOpenError(String),
}

/// Flashing status
pub enum Status {
    Preparing,
    Flashing(f32),
    Verifying,
}

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
