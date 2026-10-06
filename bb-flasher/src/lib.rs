//! # Introduction
//!
//! This crate provides common abstractions over the different flashers to be used by applications
//! such as BeagleBoard Imaging Utility. It also provides traits to add more flashers which behave
//! similar to the pre-defined ones.
//!
//! # Usage
//!
//! Flash an OS image to an SD Card (requires the `sd` feature):
//!
//! ```no_run
//! # #[cfg(feature = "sd")]
//! # fn main() {
//! use std::path::PathBuf;
//! use bb_flasher::{LocalImage, sd};
//!
//! let img = LocalImage::new(PathBuf::from("/tmp/abc.img.xz").into_boxed_path());
//! let target: sd::Target = PathBuf::from("/dev/sdb").try_into().unwrap();
//! let customization =
//!     sd::FlashingSdLinuxConfig::sysconfig(Some("beaglebone"), None, None, None, None, None, None);
//!
//! sd::Flasher::new(
//!     img.into_image_fn(),
//!     sd::NONE_BOOTFS,
//!     None::<fn() -> std::io::Result<Box<str>>>,
//!     target,
//!     customization,
//!     true,
//! )
//! .flash(None, None)
//! .unwrap();
//! # }
//! # #[cfg(not(feature = "sd"))]
//! # fn main() {}
//! ```
//!
//! # Features
//!
//! No features are enabled by default.
//!
//! - `sd`: Flash Linux images to SD Cards.
//! - `sd_linux_udev`: Uses udev to provide GUI prompt to open SD Cards in Linux. Useful for GUI
//!   applications.
//! - `sd_macos_authopen`: Uses authopen to provide GUI prompt to open SD Cards in MacOS. Useful
//!   for GUI applications.
//! - `bcf`: Flash the main processor (CC1352P7) in BeagleConnect Freedom.
//! - `bcf_msp430`: Flash MSP430 in BeagleConnect Freedom, which acts as the USB to UART bridge.
//! - `pb2_mspm0`: Flash PocketBeagle 2 MSPM0. Needs root permissions.
//! - `mspm0_uart`: Flash MSPM0 over UART.
//! - `mspm0_i2c`: Flash MSPM0 over I2C. Linux only.
//! - `dfu`: Flash devices over USB DFU.
//! - `piped_image`: Construct images from streams (e.g. a download in progress).
//! - `serde`: Implement `serde::Serialize` for some types. Enabled by `sd`.
//! - `static`: Statically link liblzma and hidraw.
//! - `static_hidraw` / `shared_hidraw`: Statically or dynamically link hidraw for
//!   BeagleConnect Freedom.

mod common;
mod flasher;
pub mod img;

use std::path::Path;

pub use common::*;
// Empty when no flasher features are enabled.
#[allow(unused_imports)]
pub use flasher::*;

/// An Os Image present in the local filesystem
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct LocalImage(Box<Path>);

impl LocalImage {
    /// Construct a new local image from path.
    pub const fn new(path: Box<Path>) -> Self {
        Self(path)
    }

    /// Path of the image.
    pub fn path(&self) -> &Path {
        &self.0
    }

    /// File name of the image. `None` if the path has no file name (e.g. `/` or `..`).
    pub fn file_name(&self) -> Option<&std::ffi::OsStr> {
        self.0.file_name()
    }

    /// Returns a function that opens the image, decompressing it if needed, and returns it along
    /// with its (uncompressed) size.
    pub fn into_image_fn(self) -> impl FnOnce() -> std::io::Result<(img::OsImage, u64)> {
        move || {
            let img = img::OsImage::from_path(&self.0)?;
            let size = img.size();

            Ok((img, size))
        }
    }

    /// Returns a function that opens the image as an [`img::OsArchive`]. Read progress is reported
    /// over `tx`.
    #[cfg(feature = "sd")]
    pub fn into_archive_fn(
        self,
        tx: Option<std::sync::mpsc::SyncSender<f32>>,
    ) -> impl FnOnce() -> std::io::Result<img::OsArchive> {
        move || img::OsArchive::from_path(&self.0, tx)
    }
}

impl std::fmt::Display for LocalImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.file_name() {
            Some(x) => write!(f, "{}", x.to_string_lossy()),
            None => write!(f, "{}", self.0.display()),
        }
    }
}
