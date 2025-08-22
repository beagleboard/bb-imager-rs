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
//!     let img = bb_flasher::LocalImage::new(PathBuf::from("/tmp/abc.img.xz").into());
//!     let target = PathBuf::from("/tmp/target").try_into().unwrap();
//!     let customization =
//!         bb_flasher::sd::FlashingSdLinuxConfig::sysconfig(None, None, None, None, None, None, None);
//!
//!     let flasher = bb_flasher::sd::Flasher::new(img, None::<bb_flasher::LocalFile>, target, customization, None)
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

use std::path::Path;

pub use common::*;
pub use flasher::*;
pub use img::OsImage;

/// A trait to signify Os Images. Flashers in this crate can take any file as an input that
/// implements this trait.
pub trait Resolvable {
    type ResolvedType: std::io::Read;

    /// Get the local path to an image. Network calls can be done here.
    fn resolve(
        &self,
    ) -> impl Future<
        Output = std::io::Result<(
            Self::ResolvedType,
            Option<tokio::task::JoinHandle<std::io::Result<()>>>,
        )>,
    >;
}

/// An Os Image present in the local filesystem
#[derive(Debug, Clone)]
pub struct LocalImage(Box<Path>);

impl LocalImage {
    /// Construct a new local image from path.
    pub const fn new(path: Box<Path>) -> Self {
        Self(path)
    }
}

impl Resolvable for LocalImage {
    type ResolvedType = OsImage;

    async fn resolve(
        &self,
    ) -> std::io::Result<(
        Self::ResolvedType,
        Option<tokio::task::JoinHandle<std::io::Result<()>>>,
    )> {
        let p = self.0.clone();
        let img = tokio::task::spawn_blocking(move || OsImage::from_path(&p))
            .await
            .unwrap()?;

        Ok((img, None))
    }
}

/// An Os Image present in the local filesystem
#[derive(Debug, Clone)]
pub struct LocalFile(Box<Path>);

impl LocalFile {
    /// Construct a new local image from path.
    pub const fn new(path: Box<Path>) -> Self {
        Self(path)
    }
}

impl Resolvable for LocalFile {
    type ResolvedType = std::fs::File;

    async fn resolve(
        &self,
    ) -> std::io::Result<(
        Self::ResolvedType,
        Option<tokio::task::JoinHandle<std::io::Result<()>>>,
    )> {
        Ok((std::fs::File::open(&self.0)?, None))
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
