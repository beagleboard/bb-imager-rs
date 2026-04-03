//! This is basically a Rust implementation of [Balena's drivelist](https://github.com/balena-io-modules/drivelist).
//!
//! - Windows
//! - Linux
//! - Macos

mod device;

mod pal;

pub use device::{DeviceDescriptor, MountPoint};
use thiserror::Error;

pub(crate) type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum Error {
    #[cfg(target_os = "linux")]
    #[error("Failed to execute lsblk.")]
    LsblkExecuteError {
        #[source]
        source: Option<std::io::Error>,
    },
    #[cfg(target_os = "windows")]
    #[error("Failed to get drive list.")]
    WindowsError {
        #[source]
        #[from]
        source: windows::core::Error,
    },
    #[cfg(target_os = "macos")]
    #[error("Failed to create DiskArbitration session")]
    MacosDiskArbitration,
}

/// Get a list of all drives
pub fn drive_list() -> crate::Result<Vec<DeviceDescriptor>> {
    pal::drive_list()
}
