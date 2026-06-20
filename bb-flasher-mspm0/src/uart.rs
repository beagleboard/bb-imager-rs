use std::sync::mpsc;
use std::time::Duration;

use bb_helper::cancel::CancellationToken;

use crate::{Error, Result, Status, helpers};

const BSL_UART_BAUD_RATE: u32 = 9600;
const BSL_UART_DATA_BITS: serialport::DataBits = serialport::DataBits::Eight;
const BSL_UART_STOP_BITS: serialport::StopBits = serialport::StopBits::One;
const BSL_UART_PARITY: serialport::Parity = serialport::Parity::None;

pub fn flash(
    firmware: &[u8],
    port: &str,
    verify: bool,
    chan: Option<mpsc::SyncSender<Status>>,
    cancel: Option<CancellationToken>,
    prep_hook: impl FnOnce() -> Result<()>,
) -> Result<()> {
    helpers::flash(
        firmware,
        || {
            let p = serialport::new(port, BSL_UART_BAUD_RATE)
                .parity(BSL_UART_PARITY)
                .stop_bits(BSL_UART_STOP_BITS)
                .data_bits(BSL_UART_DATA_BITS)
                // MSPM0 can be quite slow to respond when full length packet sent
                .timeout(Duration::from_secs(10))
                .open_native()
                .map_err(|_| Error::FailedToOpenPort)?;

            crate::bsl::Mspm0::serial(p)
        },
        verify,
        chan,
        cancel,
        prep_hook,
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
