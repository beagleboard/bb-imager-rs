//! Library to flash SD cards with OS images. Powers sd card flashing in [BeagleBoard Imager].
//!
//! Also allows optional extra [Customization] for BeagleBoard images.
//!
//! # Platform Support
//!
//! - Linux
//! - Windows
//! - MacOS
//!
//! # Features
//!
//! - `udev`: Dynamic permissions on Linux. Mostly useful for GUI and flatpaks
//! - `macos_authopen`: Dynamic permissions on MacOS.
//!
//! [BeagleBoard Imager]: https://github.com/beagleboard/bb-imager-rs

use std::{
    io,
    path::{Path, PathBuf},
};

use thiserror::Error;

pub mod bootfs_update;
pub(crate) mod customization;
mod flashing;
mod helpers;
pub(crate) mod pal;

pub use customization::{ContentType, Customization, ParitionType};
pub use flashing::flash;

pub(crate) type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Error, Debug)]
/// Errors for this crate
pub enum Error {
    /// The partition table of image invalid.
    #[error("Partition table of image not valid.")]
    InvalidPartitionTable,
    #[error("Only FAT BOOT partitions are supported.")]
    InvalidBootPartition,
    #[error("Failed to create customization {file}")]
    CustomizationFileCreateFail {
        #[source]
        source: io::Error,
        file: Box<str>,
    },
    /// Unknown error occured during IO.
    #[error("Unknown Error during IO. Please check logs for more information.")]
    IoError {
        #[from]
        #[source]
        source: io::Error,
    },
    /// Aborted before completing
    #[error("Aborted before completing.")]
    Aborted,
    #[error("Failed to format SD Card.")]
    FailedToFormat {
        #[source]
        source: io::Error,
    },
    #[error("Failed to open SD Card.")]
    FailedToOpenDestination {
        #[source]
        source: anyhow::Error,
    },
    #[error("Invalid bmap for the image.")]
    InvalidBmap,
    #[error("Writer thread has been closed.")]
    WriterClosed,

    #[cfg(windows)]
    #[error("Failed to clear SD Card.")]
    WindowsCleanError(std::process::Output),
}

/// Enumerate all SD Cards in system
pub fn devices(filter: bool) -> std::collections::HashSet<Device> {
    bb_drivelist::drive_list()
        .expect("Unsupported OS for Sd Card")
        .into_iter()
        .filter(|x| {
            if filter {
                x.is_removable && !x.is_virtual
            } else {
                true
            }
        })
        .map(|x| Device::new(x.description, x.raw.into(), x.size.unwrap_or_default()))
        .collect()
}

#[derive(Hash, Debug, PartialEq, Eq, Clone)]
/// SD Card
pub struct Device {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
}

impl Device {
    const fn new(name: String, path: PathBuf, size: u64) -> Self {
        Self { name, path, size }
    }
}

/// Format SD card to fat32
pub fn format(dst: &std::path::Path) -> Result<()> {
    crate::pal::format(dst)
}

#[derive(Debug, Clone)]
pub enum Destination {
    File(Box<Path>),
    SdCard(Box<Path>),
}
