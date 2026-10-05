//! Flash MSPM0 over UART.

use std::sync::mpsc;
use std::time::Duration;

use bb_helper::cancel::CancellationToken;

use crate::{Error, Result, Status, helpers};

const BSL_UART_BAUD_RATE: u32 = 9600;
const BSL_UART_DATA_BITS: serialport::DataBits = serialport::DataBits::Eight;
const BSL_UART_STOP_BITS: serialport::StopBits = serialport::StopBits::One;
const BSL_UART_PARITY: serialport::Parity = serialport::Parity::None;

/// Flash firmware to MSPM0 over UART.
///
/// - `firmware`: raw binary or a text format supported by [`bin_file`].
/// - `port`: serial port path, see [`ports`].
/// - `verify`: skip flashing if the device already has the same firmware, and verify the CRC
///   after flashing.
/// - `chan`: optional channel for live [`Status`] updates.
/// - `cancel`: optional token to abort flashing.
/// - `prep_hook`: called before connecting, e.g. to put MSPM0 into BSL mode.
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

/// Returns all paths to serial ports. Returns nothing if ports cannot be enumerated.
pub fn ports() -> impl Iterator<Item = String> {
    serialport::available_ports()
        .inspect_err(|e| tracing::warn!("Failed to enumerate serial ports: {e}"))
        .unwrap_or_default()
        .into_iter()
        .map(|x| x.port_name)
}
