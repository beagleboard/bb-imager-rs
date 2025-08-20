//! # Introduction
//!
//! This crate provides common abstractions over the different flashers to be used by applications
//! such as BeagleBoard Imaging Utility. It also provides traits to add more flashers which behave
//! similiar to the pre-defined ones
//!
//! # Usage
//!
//! ```no_run
//! use std::path::{PathBuf, Path};
//! use bb_flasher::BBFlasher;
//!
//! #[tokio::main]
//! async fn main() {
//!     let img = bb_flasher::OsImage::from_path(Path::new("/tmp/abc.img.xz")).unwrap();
//!     let target = PathBuf::from("/tmp/target").try_into().unwrap();
//!     let customization =
//!         bb_flasher::sd::FlashingSdLinuxConfig::sysconfig(None, None, None, None, None, None, None);
//!
//!     let flasher = bb_flasher::sd::Flasher::new(img, None::<bb_flasher::LocalImage>, target, customization)
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

pub use common::*;
pub use flasher::*;
pub use img::{OsImage, ReadPipe};
