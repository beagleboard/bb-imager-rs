//! Library to flash SD cards with OS images. Powers sd card flashing in [BeagleBoard Imager].
//!
//! Also allows optional extra [Customization] for BeagleBoard images. Currently only supports
//! sysconf based post-install configuration.
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
//! # Usage
//!
//! ```no_run
//! use std::path::Path;
//! use std::fs::File;
//!
//! let dst = Path::new("/tmp/dummy");
//! let img = || {
//!     Ok((File::open("/tmp/image")?, 1024))
//! };
//! let (tx, rx) = futures::channel::mpsc::channel(20);
//!
//! let flash_thread = std::thread::spawn(move || {
//!     bb_flasher_sd::flash(
//!         img,
//!         dst,
//!         true,
//!         Some(tx),
//!         None,
//!         None
//!     )
//! });
//!
//! let msgs = futures::executor::block_on_stream(rx);
//! for m in msgs {
//!     println!("{:?}", m);
//! }
//!
//! flash_thread.join().unwrap().unwrap()
//! ```
//!
//! [BeagleBoard Imager]: https://openbeagle.org/ayush1325/bb-imager-rs

use std::{io, path::PathBuf};

use thiserror::Error;

pub(crate) mod customization;
mod flashing;
mod helpers;
pub(crate) mod pal;

pub use customization::Customization;
pub use flashing::flash;

pub(crate) type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Error, Debug)]
/// Errors for this crate
pub enum Error {
    #[error("Sha256 verification error")]
    Sha256Verification,
    #[error("Failed to customize flashed image {0}")]
    Customization(String),
    #[error("IO Error: {0}")]
    IoError(io::Error),
    /// Aborted before completing
    #[error("Aborted before completing")]
    Aborted,
    #[error("Failed to format SD Card: {0}")]
    FailedToFormat(String),
    #[error("Failed to open {0}")]
    FailedToOpenDestination(String),

    #[error("Udisks2 Error: {0}")]
    #[cfg(feature = "udev")]
    Udisks(#[from] udisks2::Error),

    #[cfg(windows)]
    #[error("Drive path is not valid")]
    InvalidDrive,
    #[cfg(windows)]
    #[error("Failed to find the drive {0}")]
    DriveNotFound(String),
    #[cfg(windows)]
    #[error("Windows Error: {0}")]
    WindowsError(#[from] windows::core::Error),
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::IoError(value)
    }
}

/// Enumerate all SD Cards in system
pub fn devices() -> std::collections::HashSet<Device> {
    bb_drivelist::drive_list()
        .expect("Unsupported OS for Sd Card")
        .into_iter()
        .filter(|x| x.is_removable)
        .filter(|x| !x.is_virtual)
        .map(|x| Device::new(x.description, x.raw.into(), x.size))
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

/// Flashing status
#[derive(Clone, Debug)]
pub enum Status {
    Preparing,
    Flashing(f32),
    Verifying(f32),
}
