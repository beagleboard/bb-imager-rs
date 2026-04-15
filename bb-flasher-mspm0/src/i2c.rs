use std::path::{Path, PathBuf};

use i2cdev::core::I2CDevice;
use tokio::sync::mpsc;

use crate::{Error, Result, Status};

const BSL_TARGET_ADDRESS: u16 = 0x48;
const BSL_ACK: u8 = 0x00;
const BSL_CONNECTION_REQ: [u8; 8] = [0x80, 0x01, 0x00, 0x12, 0x3A, 0x61, 0x44, 0xDE];

struct I2CDev(i2cdev::linux::LinuxI2CDevice);

impl I2CDev {
    fn new(port: &Path) -> Result<Self> {
        i2cdev::linux::LinuxI2CDevice::new(port, BSL_TARGET_ADDRESS)
            .map_err(|_| Error::FailedToOpenPort)
            .map(Self)
    }
}

impl std::io::Read for I2CDev {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf).map_err(std::io::Error::other)?;
        Ok(buf.len())
    }
}

impl std::io::Write for I2CDev {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf).map_err(std::io::Error::other)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub fn flash(
    firmware: &[u8],
    port: &Path,
    verify: bool,
    chan: Option<mpsc::Sender<Status>>,
    cancel: Option<tokio_util::sync::CancellationToken>,
) -> Result<()> {
    crate::helpers::flash(firmware, || I2CDev::new(port), verify, chan, cancel)
}

fn probe_port(port: &Path) -> bool {
    let mut dev = match i2cdev::linux::LinuxI2CDevice::new(port, BSL_TARGET_ADDRESS) {
        Ok(dev) => dev,
        Err(_) => return false,
    };

    if dev.write(&BSL_CONNECTION_REQ).is_err() {
        return false;
    }

    let mut ack = [0u8; 1];
    if dev.read(&mut ack).is_err() {
        return false;
    }

    ack[0] == BSL_ACK
}

/// Returns all paths to serial ports.
pub fn ports() -> std::collections::HashSet<PathBuf> {
    std::fs::read_dir("/dev")
        .unwrap()
        .filter_map(|x| x.ok())
        // chardev is not reported as file.
        .filter(|x| match x.file_type() {
            Ok(y) => !y.is_dir(),
            Err(_) => false,
        })
        .filter(|x| match x.file_name().to_str() {
            Some(y) => y.contains("i2c-"),
            None => false,
        })
        .map(|x| x.path())
        .filter(|x| probe_port(x))
        .collect()
}
