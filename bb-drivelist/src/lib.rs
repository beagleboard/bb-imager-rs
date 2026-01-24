//! This is basically a Rust implementation of [Balena's drivelist](https://github.com/balena-io-modules/drivelist).
//!
//! - Windows
//! - Linux
//! - Macos

mod device;

mod pal;

pub use device::{DeviceDescriptor, MountPoint};

/// Get a list of all drives
pub fn drive_list() -> anyhow::Result<Vec<DeviceDescriptor>> {
    pal::drive_list()
}
