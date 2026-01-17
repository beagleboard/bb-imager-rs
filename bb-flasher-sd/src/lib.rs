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
//! use std::path::PathBuf;
//! use std::fs::File;
//!
//! #[tokio::main]
//! async fn main() {
//!     let dst = PathBuf::from("/tmp/dummy").into();
//!     let img = bb_helper::resolvable::LocalFile::new(PathBuf::from("/tmp/image").into());
//!     let (tx, mut rx) = tokio::sync::mpsc::channel(20);
//!
//!     let flash_thread = tokio::spawn(async move { bb_flasher_sd::flash(img, None::<bb_helper::resolvable::LocalStringFile>, dst, Some(tx), None, None).await });
//!
//!     while let Some(m) = rx.recv().await {
//!         println!("{:?}", m);
//!     }
//!
//!     flash_thread.await.unwrap().unwrap()
//! }
//! ```
//!
//! [BeagleBoard Imager]: https://openbeagle.org/ayush1325/bb-imager-rs

use std::{io, path::PathBuf};

use thiserror::Error;

pub(crate) mod customization;
mod flashing;
mod helpers;
pub(crate) mod pal;

pub use customization::{Customization, SysconfCustomization};
pub use flashing::flash;

pub(crate) type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Error, Debug)]
/// Errors for this crate
pub enum Error {
    /// Provided customization options are not valid for the current image.
    #[error("Invalid customization options.")]
    InvalidCustomizaton,
    /// The partition table of image invalid.
    #[error("Partition table of image not valid.")]
    InvalidPartitionTable,
    #[error("Only FAT BOOT partitions are supported.")]
    InvalidBootPartition,
    #[error("Failed to create sysconf.txt")]
    SysconfCreateFail {
        #[source]
        source: io::Error,
    },
    #[error("Failed to write {field} to sysconf.txt.")]
    SysconfWriteFail {
        #[source]
        source: io::Error,
        field: &'static str,
    },
    #[error("Failed to setup WiFi.")]
    WifiSetupFail {
        #[source]
        source: io::Error,
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
pub fn devices() -> std::collections::HashSet<Device> {
    let mut devices = std::collections::HashSet::new();

    // Primary method: use bb-drivelist
    if let Ok(drives) = bb_drivelist::drive_list() {
        for drive in drives {
            if drive.is_removable && !drive.is_virtual {
                devices.insert(Device::new(
                    drive.description,
                    drive.raw.into(),
                    drive.size,
                ));
            }
        }
    }

    // Fallback for macOS: use diskutil to detect external drives
    // This helps with macOS Tahoe 26.1+ where bb-drivelist may miss drives
    // due to new security restrictions
    #[cfg(target_os = "macos")]
    {
        if let Ok(diskutil_devices) = get_devices_from_diskutil() {
            for device in diskutil_devices {
                devices.insert(device);
            }
        }
    }

    devices
}

#[cfg(target_os = "macos")]
fn get_devices_from_diskutil() -> Result<std::collections::HashSet<Device>, io::Error> {
    use std::process::Command;

    let mut devices = std::collections::HashSet::new();

    // Run diskutil list to get all disks in plist format
    let output = Command::new("diskutil")
        .args(["list", "-plist"])
        .output()?;

    if !output.status.success() {
        return Ok(devices);
    }

    // Parse plist output
    let plist_data = output.stdout;
    if let Ok(plist) = plist::Value::from_reader(&plist_data[..]) {
        if let Some(dict) = plist.as_dictionary() {
            if let Some(disks) = dict.get("AllDisksAndPartitions") {
                if let Some(disk_array) = disks.as_array() {
                    for disk in disk_array {
                        if let Some(disk_dict) = disk.as_dictionary() {
                            // Check if it's an external/removable disk
                            // Internal disks have "Internal" = true, external have false or missing
                            let is_internal = disk_dict
                                .get("Internal")
                                .and_then(|v| v.as_boolean())
                                .unwrap_or(false);

                            if !is_internal {
                                // Get device identifier (e.g., disk2)
                                if let Some(device_identifier) = disk_dict
                                    .get("DeviceIdentifier")
                                    .and_then(|v| v.as_string())
                                {
                                    // Skip if it's a partition (partitions have numbers like disk2s1)
                                    if device_identifier.contains('s') {
                                        continue;
                                    }

                                    // Get size
                                    let size = disk_dict
                                        .get("Size")
                                        .and_then(|v| v.as_integer())
                                        .unwrap_or(0) as u64;

                                    // Get volume name or use device identifier
                                    let name = disk_dict
                                        .get("VolumeName")
                                        .and_then(|v| v.as_string())
                                        .map(|s| s.to_string())
                                        .unwrap_or_else(|| {
                                            format!("External Disk {}", device_identifier)
                                        });

                                    // Construct path: /dev/rdiskX for raw access
                                    let disk_num = device_identifier
                                        .strip_prefix("disk")
                                        .and_then(|s| s.parse::<u32>().ok())
                                        .unwrap_or(0);

                                    if disk_num > 0 {
                                        let path = PathBuf::from(format!("/dev/rdisk{}", disk_num));

                                        // Verify the device exists and is accessible
                                        if path.exists() {
                                            devices.insert(Device::new(name, path, size));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(devices)
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
pub async fn format(dst: &std::path::Path) -> Result<()> {
    crate::pal::format(dst).await
}
