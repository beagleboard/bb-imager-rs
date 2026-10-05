//! Flash MSPM0 over I2C. Linux only.

use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use bb_helper::cancel::CancellationToken;
use i2cdev::core::I2CDevice;

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

/// Flash firmware to MSPM0 over I2C.
///
/// - `firmware`: raw binary or a text format supported by [`bin_file`].
/// - `port`: I2C bus device path, see [`ports`].
/// - `verify`: skip flashing if the device already has the same firmware, and verify the CRC
///   after flashing.
/// - `chan`: optional channel for live [`Status`] updates.
/// - `cancel`: optional token to abort flashing.
/// - `prep_hook`: called before connecting, e.g. to put MSPM0 into BSL mode (see
///   [`crate::bsl_gpio_cdev_by_name`]).
pub fn flash(
    firmware: &[u8],
    port: &Path,
    verify: bool,
    chan: Option<mpsc::SyncSender<Status>>,
    cancel: Option<CancellationToken>,
    prep_hook: impl FnOnce() -> Result<()>,
) -> Result<()> {
    crate::helpers::flash(
        firmware,
        || {
            let d = I2CDev::new(port)?;
            crate::bsl::Mspm0::new(d)
        },
        verify,
        chan,
        cancel,
        prep_hook,
    )
}

/// Returns all paths to I2C bus devices. Returns nothing if `/dev` cannot be read.
pub fn ports() -> impl Iterator<Item = PathBuf> {
    std::fs::read_dir("/dev")
        .inspect_err(|e| tracing::warn!("Failed to read /dev: {e}"))
        .into_iter()
        .flatten()
        .filter_map(|x| x.ok())
        .filter(|x| {
            matches!(
                x.metadata().map(|m| nix::sys::stat::major(m.rdev()) == 89),
                Ok(true)
            )
        })
        .map(|x| x.path())
}
