use tokio::sync::mpsc;

use std::time::Duration;

use crate::{Error, Result, Status, helpers};

const BSL_UART_BAUD_RATE: u32 = 9600;
const BSL_UART_DATA_BITS: serialport::DataBits = serialport::DataBits::Eight;
const BSL_UART_STOP_BITS: serialport::StopBits = serialport::StopBits::One;
const BSL_UART_PARITY: serialport::Parity = serialport::Parity::None;

pub fn flash(
    firmware: &[u8],
    port: &str,
    verify: bool,
    chan: Option<mpsc::Sender<Status>>,
    cancel: Option<tokio_util::sync::CancellationToken>,
) -> Result<()> {
    helpers::flash(
        firmware,
        || {
            serialport::new(port, BSL_UART_BAUD_RATE)
                .parity(BSL_UART_PARITY)
                .stop_bits(BSL_UART_STOP_BITS)
                .data_bits(BSL_UART_DATA_BITS)
                .timeout(Duration::from_secs(5))
                .open_native()
                .map_err(|_| Error::FailedToOpenPort)
        },
        verify,
        chan,
        cancel,
    )
}

/// Returns all paths to serial ports.
pub fn ports() -> std::collections::HashSet<String> {
    serialport::available_ports()
        .expect("Unsupported OS")
        .into_iter()
        .map(|x| x.port_name)
        .collect()
}
