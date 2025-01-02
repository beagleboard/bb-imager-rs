//! Helpers to enable flashing BeagleConnect Freedom MSP430 firmware

use std::time::Duration;

use crate::{error::Result, util};
use thiserror::Error;

const VID: u16 = 0x2047;
const PID: u16 = 0x0200;

const USB_MSG_HEADER: u8 = 0x3f;

const COMMAND_MAX_SIZE: usize = 48;

const BSL: &str = include_str!("../../assets/MSP430_BSL.00.06.05.34.txt");
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

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to Write: {0}")]
    FailedToWrite(String),
    #[error("Failed to Read: {0}")]
    FailedToRead(String),
}

struct MSP430(hidapi::HidDevice);

impl MSP430 {
    fn request(cmd: u8, data: &[u8]) -> Vec<u8> {
        [USB_MSG_HEADER, (data.len() + 1) as u8, cmd]
            .into_iter()
            .chain(data.iter().cloned())
            .collect()
    }

    fn cmd_no_resp(&self, cmd: u8, data: &[u8]) -> Result<()> {
        let req = Self::request(cmd, data);

        self.0
            .write(&req)
            .map(|_| ())
            .map_err(|e| Error::FailedToWrite(e.to_string()))
            .map_err(Into::into)
    }

    fn cmd(&self, cmd: u8, data: &[u8]) -> Result<Vec<u8>> {
        let mut ans = [0u8; 256];

        let req = Self::request(cmd, data);
        self.0
            .write(&req)
            .map_err(|e| Error::FailedToWrite(e.to_string()))?;

        let _ = self
            .0
            .read(&mut ans)
            .map_err(|e| Error::FailedToRead(e.to_string()))?;

        assert_eq!(ans[0], USB_MSG_HEADER);
        let length = ans[1];

        Ok(ans[2..(2 + length as usize)].to_vec())
    }

    fn mass_erase(&self) -> Result<()> {
        let ans = self.cmd(
            CMD_RX_PASSWORD,
            &[
                0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                0xff, 0xff, 0, 0,
            ],
        )?;

        assert_eq!(ans.len(), 2);
        assert_ne!(ans[1], 0);

        Ok(())
    }

    fn unlock(&self) -> Result<()> {
        let ans = self.cmd(CMD_RX_PASSWORD, &[0xffu8; 32])?;

        assert_eq!(ans.len(), 2);
        assert_eq!(ans[1], 0);

        Ok(())
    }

    fn load_pc(&self) -> Result<()> {
        self.cmd_no_resp(CMD_LOAD_PC, &BSL_START_ADDR)
    }

    fn bsl_version(&self) -> Result<()> {
        let resp = self.cmd(CMD_TX_BSL_VERSION, &[])?;

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

        self.cmd_no_resp(CMD_RX_DATA_BLOCK_FAST, &data)?;

        Ok(bytes_to_write)
    }

    fn load_binfile(&self, bin: &bin_file::BinFile) -> Result<()> {
        for (start_address, mut data) in bin.segments_list() {
            let mut offset = 0;

            // Pad
            if data.len() & 1 != 0 {
                data.push(0xff);
            }

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

    let bin = util::bin_file_from_str(BSL).expect("Failed to parse MSP430 BSL");
    tracing::info!("BSL: {}", bin);

    tracing::info!("Load BSL");
    msp430.load_binfile(&bin)?;

    tracing::info!("Load PC");
    msp430.load_pc()?;

    Ok(())
}

pub fn flash(
    img: bin_file::BinFile,
    dst: &std::ffi::CStr,
    chan: &tokio::sync::mpsc::Sender<crate::DownloadFlashingStatus>,
) -> Result<()> {
    let _ = chan.try_send(crate::DownloadFlashingStatus::Preparing);

    load_bsl(dst)?;

    std::thread::sleep(Duration::from_secs(1));

    let _ = chan.try_send(crate::DownloadFlashingStatus::FlashingProgress(0.5));

    let msp430 = MSP430(open_hidraw(dst)?);

    tracing::info!("Get BSL Version");
    msp430.bsl_version()?;
    tracing::info!("Flashing");
    msp430.load_binfile(&img)?;

    Ok(())
}

pub fn possible_devices() -> std::collections::HashSet<crate::Destination> {
    hidapi::HidApi::new()
        .expect("Failed to create hidapi context")
        .device_list()
        .filter(|x| x.vendor_id() == VID && x.product_id() == PID)
        .map(|x| crate::Destination::hidraw(x.path().to_owned()))
        .collect()
}

pub fn open_hidraw(dst: &std::ffi::CStr) -> crate::error::Result<hidapi::HidDevice> {
    hidapi::HidApi::new()
        .map_err(|e| crate::Error::FailedToOpenDestination(e.to_string()))?
        .open_path(dst)
        .map_err(|e| crate::Error::FailedToOpenDestination(e.to_string()))
        .map_err(Into::into)
}
