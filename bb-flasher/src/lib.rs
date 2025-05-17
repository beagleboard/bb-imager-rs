//! # Introduction
//!
//! This crate provides common abstractions over the different flashers to be used by applications
//! such as BeagleBoard Imaging Utility. It also provides traits to add more flashers which behave
//! similiar to the pre-defined ones
//!
//! # Usage
//!
//! ```no_run
//! use std::path::PathBuf;
//! use bb_flasher::BBFlasher;
//!
//! #[tokio::main]
//! async fn main() {
//!     let img = bb_flasher::LocalImage::new("/tmp/abc.img.xz".into());
//!     let target = PathBuf::from("/tmp/target").try_into().unwrap();
//!     let customization = 
//!         bb_flasher::sd::FlashingSdLinuxConfig::new(None, None, None, None, None, None);
//!
//!     let flasher = bb_flasher::sd::Flasher::new(img, target, customization)
//!         .flash(None)
//!         .await
//!         .unwrap();
//! }
//! ```
//!
//! # Features
//!
//! - `sd`: Provide flashing Linux images to SD Cards. Enabled by **default**.
//! - `sd_linux_udev`: Uses udev to provide GUI prompt to open SD Cards in Linux. Useful for GUI
//!   applications.
//! - `sd_macos_authopen`: Uses authopen to provide GUI prompt to open SD Cards in MacOS. Useful
//!   for GUI applications.
//! - `bcf`: Provde support for flashing the main processor (CC1352P7) in BeagleConnect Freedom.
//! - `bcf_msp430`: Provide support for flashing MSP430 in BeagleConnect Freedom, which acts as the
//!   USB to UART bridge.
//! - `pb2_mspm0`: Provides support to flash PocketBeagle 2 MSPM0. Needs root permissions.
//! - `pb2_mspm0_dbus`: Use bb-imager-serivce to flash PocketBeagle 2 as a normal user.

mod common;
mod flasher;
mod img;

use std::path::PathBuf;

pub use common::*;
pub use flasher::*;
use futures::channel::mpsc;

/// A trait to signify Os Images. Flashers in this crate can take any file as an input that
/// implements this trait.
pub trait ImageFile {
    /// Get the local path to an image. Network calls can be done here.
    fn resolve(
        &self,
        chan: Option<mpsc::Sender<DownloadFlashingStatus>>,
    ) -> impl Future<Output = std::io::Result<PathBuf>>;
}

/// An Os Image present in the local filesystem
#[derive(Debug, Clone)]
pub struct LocalImage(PathBuf);

impl LocalImage {
    /// Construct a new local image from path.
    pub const fn new(path: PathBuf) -> Self {
        Self(path)
    }
}

impl ImageFile for LocalImage {
    fn resolve(
        &self,
        _: Option<mpsc::Sender<DownloadFlashingStatus>>,
    ) -> impl Future<Output = std::io::Result<PathBuf>> {
        std::future::ready(Ok(self.0.clone()))
    }
}

impl std::fmt::Display for LocalImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .file_name()
                .expect("image cannot be a directory")
                .to_string_lossy()
        )
    }
}
