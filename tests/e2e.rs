//! End-to-End Testing for BeagleBoard Imager
//!
//! This module contains comprehensive E2E tests for all flashing targets:
//! - SD Card flashing (Linux, Windows, macOS)
//! - BeagleConnect Freedom (BCF) CC1352P7 flashing
//! - BeagleConnect Freedom MSP430 flashing
//! - DFU (Device Firmware Update) flashing
//!
//! ## Running Tests
//!
//! Run all E2E tests:
//! ```bash
//! cargo test --test e2e
//! ```
//!
//! Run specific platform tests:
//! ```bash
//! cargo test --test e2e sd_flash  # SD card tests
//! cargo test --test e2e bcf_flash  # BCF tests
//! cargo test --test e2e dfu_flash  # DFU tests
//! ```
//!
//! Run with specific features:
//! ```bash
//! cargo test --test e2e --features bcf,bcf_msp430,dfu
//! ```
//!
//! ## Platform-Specific Notes
//!
//! ### Linux
//! - SD card tests may require elevated privileges for real device access
//! - Virtual device tests run without privileges
//!
//! ### Windows
//! - Ensure proper USB drivers are installed for BCF and DFU devices
//!
//! ### macOS
//! - May require security permissions for device access
//!

mod common;

#[cfg(feature = "sd")]
mod sd_flash;

#[cfg(any(feature = "bcf", feature = "bcf_msp430"))]
mod bcf_flash;

#[cfg(feature = "dfu")]
mod dfu_flash;

