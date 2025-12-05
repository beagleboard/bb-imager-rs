//! Library to provide flashing capabilities for [MSP430F5503] in [BeagleConnect Freedom]. This is
//! the co-processor that acts as USB-to-UART.
//!
//! To flash MSP430, we need to boot into BSL. This can be done by holding BOOT button while
//! connecting the USB to [BeagleConnect Freedom].
//!
//! BSL command details can be found in [MSP430™ Flash Devices Bootloader (BSL)].
//!
//! [BeagleConnect Freedom]: https://www.beagleboard.org/boards/beagleconnect-freedom
//! [MSP430F5503]: https://www.ti.com/product/MSP430F5503
//! [MSP430™ Flash Devices Bootloader (BSL)]: https://www.ti.com/lit/ug/slau319af/slau319af.pdf?ts=1741178254884

use std::{ffi::CString, time::Duration};

use thiserror::Error;
use tokio::sync::mpsc;

use crate::{
    Status,
    helpers::{chan_send, parse_bin},
};

const VID: u16 = 0x2047;
const PID: u16 = 0x0200;

const USB_MSG_HEADER: u8 = 0x3f;

const COMMAND_MAX_SIZE: usize = 48;

const BSL: &str = include_str!("../assets/MSP430_BSL.00.06.05.34.txt");
const BSL_VERSION: [u8; 4] = [0, 0x06, 0x05, 0x34];
const BSL_START_ADDR: [u8; 3] = three_bytes(0x2504);

const CMD_RX_DATA_BLOCK_FAST: u8 = 0x1b;
const CMD_RX_PASSWORD: u8 = 0x11;
const CMD_LOAD_PC: u8 = 0x17;
const CMD_TX_BSL_VERSION: u8 = 0x19;

const fn three_bytes(x: usize) -> [u8; 3] {
    let temp = x.to_le_bytes();
    [temp[0], temp[1], temp[2]]
}

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Error, Debug)]
/// Errors for MSP430F5503
pub enum Error {
    /// Could not unlock the BSL. Maybe a custom password is being used.
    #[error("Failed to unlock BSL.")]
    UnlockFail,
    /// Failed to erase flash.
    #[error("Failed to perform mass erase.")]
    MassEraseFail,
    /// LOAD PC instruction to jump to full BSL in RAM failed.
    #[error("Failed to start full BSL.")]
    BSLJumpFail,
    /// BSL version request failed.
    #[error("Failed to read BSL Version.")]
    BslVersionFail,
    // Firmware is not valid.
    #[error("Firmware is not valid")]
    InvalidFirmware,
    /// Failed while sending firmware to BSL.
    #[error("Failed to write firmware. See logs for more info.")]
    FirmwareWriteFail {
        #[source]
        source: hidapi::HidError,
    },
    /// Failed to open MSP430.
    #[error("Failed to open MSP430")]
    FailedToOpenDestination {
        #[source]
        source: hidapi::HidError,
    },
}

struct MSP430(hidapi::HidDevice);

impl MSP430 {
    fn request(cmd: u8, data: &[u8]) -> Vec<u8> {
        [USB_MSG_HEADER, (data.len() + 1) as u8, cmd]
            .into_iter()
            .chain(data.iter().cloned())
            .collect()
    }

    fn cmd_no_resp(&self, cmd: u8, data: &[u8]) -> hidapi::HidResult<()> {
        let req = Self::request(cmd, data);

        self.0.write(&req).map(|_| ())
    }

    fn cmd(&self, cmd: u8, data: &[u8]) -> hidapi::HidResult<Vec<u8>> {
        let mut ans = [0u8; 256];

        let req = Self::request(cmd, data);
        self.0.write(&req)?;

        let _ = self.0.read(&mut ans)?;

        assert_eq!(ans[0], USB_MSG_HEADER);
        let length = ans[1];

        Ok(ans[2..(2 + length as usize)].to_vec())
    }

    fn mass_erase(&self) -> Result<()> {
        let ans = self
            .cmd(
                CMD_RX_PASSWORD,
                &[
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                    0xff, 0xff, 0xff, 0xff, 0, 0,
                ],
            )
            .map_err(|_| Error::MassEraseFail)?;

        assert_eq!(ans.len(), 2);
        assert_ne!(ans[1], 0);

        Ok(())
    }

    fn unlock(&self) -> Result<()> {
        let ans = self
            .cmd(CMD_RX_PASSWORD, &[0xffu8; 32])
            .map_err(|_| Error::UnlockFail)?;

        assert_eq!(ans.len(), 2);
        assert_eq!(ans[1], 0);

        Ok(())
    }

    fn load_pc(&self) -> Result<()> {
        self.cmd_no_resp(CMD_LOAD_PC, &BSL_START_ADDR)
            .map_err(|_| Error::BSLJumpFail)
    }

    fn bsl_version(&self) -> Result<()> {
        let resp = self
            .cmd(CMD_TX_BSL_VERSION, &[])
            .map_err(|_| Error::BslVersionFail)?;

        assert_eq!(resp[0], 0x3a);
        assert_eq!(resp[1..], BSL_VERSION);

        Ok(())
    }

    fn rx_data_block_fast(&self, addr: usize, block: &[u8]) -> Result<usize> {
        let bytes_to_write = std::cmp::min(block.len(), COMMAND_MAX_SIZE);

        let addr = three_bytes(addr);
        let data: Vec<u8> = addr
            .into_iter()
            .chain(block[..bytes_to_write].iter().cloned())
            .collect();

        self.cmd_no_resp(CMD_RX_DATA_BLOCK_FAST, &data)
            .map_err(|e| Error::FirmwareWriteFail { source: e })?;

        Ok(bytes_to_write)
    }

    fn load_binfile(&self, bin: &bin_file::BinFile) -> Result<()> {
        for (start_address, data) in bin.segments_list() {
            tracing::debug!(
                "Start Address: {}, Data: {:?}, Data Len: {}",
                start_address,
                data,
                data.len()
            );
            let mut offset = 0;
            // assert!(data.len() % 2 == 0);
            while offset < data.len() {
                offset += self.rx_data_block_fast(start_address + offset, &data[offset..])?;
            }
        }

        Ok(())
    }
}

fn load_bsl(dst: &std::ffi::CStr) -> Result<()> {
    let msp430 = MSP430(open_hidraw(dst)?);

    tracing::info!("Mass Erase");
    msp430.mass_erase()?;

    std::thread::sleep(Duration::from_secs(1));

    tracing::info!("Unlock");
    msp430.unlock()?;

    let bin = BSL.parse().expect("Failed to parse MSP430 BSL");
    tracing::info!("BSL: {}", bin);

    tracing::info!("Load BSL");
    msp430.load_binfile(&bin)?;

    tracing::info!("Load PC");
    msp430.load_pc()?;

    Ok(())
}

/// Flash MSP430 in BeagleConnect Freedom. Provides optional progress.
///
/// # Firmware
///
/// Firmware type is auto detected. Supported firmwares:
///
/// - Raw binary
/// - Ti-TXT
/// - Intel Hex
///
/// No abort mechanism is provided here since the time taken to flash is ~2 secs. So aborting is
/// not much useful other than stress tests.
pub fn flash(
    firmware: &[u8],
    dst: &std::ffi::CStr,
    mut chan: Option<mpsc::Sender<Status>>,
) -> Result<()> {
    let firmware_bin = parse_bin(firmware).map_err(|_| Error::InvalidFirmware)?;

    chan_send(chan.as_mut(), Status::Preparing);

    load_bsl(dst)?;

    std::thread::sleep(Duration::from_secs(1));

    chan_send(chan.as_mut(), Status::Flashing(0.5));

    let msp430 = MSP430(open_hidraw(dst)?);

    tracing::info!("Get BSL Version");
    msp430.bsl_version()?;
    tracing::info!("Flashing");
    msp430.load_binfile(&firmware_bin)?;

    Ok(())
}

/// Returns all paths to ports having BeagleConnect Freedom.
pub fn devices() -> std::collections::HashSet<CString> {
    hidapi::HidApi::new()
        .expect("Failed to create hidapi context")
        .device_list()
        .filter(|x| x.vendor_id() == VID && x.product_id() == PID)
        .map(|x| x.path().to_owned())
        .collect()
}

fn open_hidraw(dst: &std::ffi::CStr) -> Result<hidapi::HidDevice> {
    hidapi::HidApi::new()
        .map_err(|source| Error::FailedToOpenDestination { source })?
        .open_path(dst)
        .map_err(|source| Error::FailedToOpenDestination { source })
}
