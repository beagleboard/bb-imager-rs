use std::{
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

use i2cdev::core::I2CDevice;
use tokio::sync::mpsc;

use crate::{Error, Result, Status};

const BSL_TARGET_ADDRESS: u16 = 0x48;

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
    prep_hook: impl FnOnce() -> Result<()>,
) -> Result<()> {
    crate::helpers::flash(
        firmware,
        || I2CDev::new(port),
        verify,
        chan,
        cancel,
        prep_hook,
    )
}

/// Returns all paths to serial ports.
pub fn ports() -> std::collections::HashSet<PathBuf> {
    std::fs::read_dir("/dev")
        .unwrap()
        .filter_map(|x| x.ok())
        .filter(|x| {
            matches!(
                x.metadata().map(|m| nix::sys::stat::major(m.rdev()) == 89),
                Ok(true)
            )
        })
        .map(|x| x.path())
        .collect()
}
