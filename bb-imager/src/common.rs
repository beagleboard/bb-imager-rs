//! Stuff common to all the flashers

use std::{
    collections::HashSet,
    ffi::CString,
    io::{Read, SeekFrom},
    path::PathBuf,
};
use thiserror::Error;
use tokio::io::AsyncSeekExt;

use crate::{
    flasher::{bcf, msp430, sd},
    util,
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to Open Destination: {0}")]
    FailedToOpenDestination(String),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DownloadFlashingStatus {
    Preparing,
    DownloadingProgress(f32),
    FlashingProgress(f32),
    Verifying,
    VerifyingProgress(f32),
    Customizing,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum Destination {
    Port(String),
    SdCard {
        name: String,
        path: String,
        size: u64,
    },
    HidRaw(std::ffi::CString),
    File(String, PathBuf),
}

impl Destination {
    pub const fn port(name: String) -> Self {
        Self::Port(name)
    }

    pub const fn size(&self) -> u64 {
        if let Self::SdCard { size, .. } = self {
            *size
        } else {
            0
        }
    }

    pub const fn sd_card(name: String, size: u64, path: String) -> Self {
        Self::SdCard { name, path, size }
    }

    pub const fn hidraw(path: CString) -> Self {
        Self::HidRaw(path)
    }

    pub const fn file(name: String, path: PathBuf) -> Self {
        Self::File(name, path)
    }

    pub fn path(&self) -> PathBuf {
        match self {
            Destination::Port(p) => PathBuf::from(p),
            Destination::SdCard { path, .. } => PathBuf::from(path),
            Destination::HidRaw(p) => PathBuf::from(p.to_str().unwrap()),
            Destination::File(_, p) => p.to_path_buf(),
        }
    }
}

impl std::fmt::Display for Destination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Destination::Port(p) => write!(f, "{}", p),
            Destination::SdCard { name, .. } => write!(f, "{}", name),
            Destination::HidRaw(p) => write!(f, "{}", p.to_string_lossy()),
            Destination::File(n, _) => write!(f, "{}", n),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectedImage {
    Local(PathBuf),
    Remote {
        name: String,
        url: url::Url,
        extract_sha256: [u8; 32],
    },
}

impl SelectedImage {
    pub const fn local(name: PathBuf) -> Self {
        Self::Local(name)
    }

    pub const fn remote(name: String, url: url::Url, download_sha256: [u8; 32]) -> Self {
        Self::Remote {
            name,
            url,
            extract_sha256: download_sha256,
        }
    }
}

impl std::fmt::Display for SelectedImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectedImage::Local(p) => write!(
                f,
                "{}",
                p.file_name()
                    .expect("image cannot be a directory")
                    .to_string_lossy()
            ),
            SelectedImage::Remote { name, .. } => write!(f, "{}", name),
        }
    }
}

#[cfg(feature = "config")]
impl From<&crate::config::OsImage> for SelectedImage {
    fn from(value: &crate::config::OsImage) -> Self {
        Self::remote(
            value.name.clone(),
            value.url.clone(),
            value.image_download_sha256,
        )
    }
}

pub enum FlashingConfig {
    LinuxSdFormat {
        dst: String,
    },
    LinuxSd {
        img: SelectedImage,
        dst: String,
        customization: sd::FlashingSdLinuxConfig,
    },
    BeagleConnectFreedom {
        img: SelectedImage,
        port: String,
        customization: bcf::FlashingBcfConfig,
    },
    Msp430 {
        img: SelectedImage,
        port: std::ffi::CString,
    },
    #[cfg(any(feature = "pb2_mspm0_raw", feature = "pb2_mspm0_dbus"))]
    Pb2Mspm0 {
        img: SelectedImage,
        persist_eeprom: bool,
    },
}

impl FlashingConfig {
    pub async fn download_flash_customize(
        self,
        downloader: crate::download::Downloader,
        chan: tokio::sync::mpsc::Sender<DownloadFlashingStatus>,
    ) -> crate::error::Result<()> {
        match self {
            FlashingConfig::LinuxSdFormat { dst } => sd::format(&dst).await,
            FlashingConfig::LinuxSd {
                img,
                dst,
                customization,
            } => {
                let mut disk = sd::open(&dst).await?;
                let img = crate::img::OsImage::from_selected_image(img, &downloader, &chan).await?;

                sd::flash(img, &mut disk, &chan, customization.verify).await?;

                // Performing check inside customize seems to cause weird behaviour. Maybe
                // spawn_blocking overhead is substantial
                if customization.has_customization() {
                    let _ = chan.try_send(DownloadFlashingStatus::Customizing);
                    disk.seek(SeekFrom::Start(0)).await?;
                    let mut std_disk = disk.into_std().await;
                    tokio::task::spawn_blocking(move || customization.customize(&mut std_disk))
                        .await
                        .expect("Tokio runtime failed to spawn blocking task")?;
                }

                Ok(())
            }
            FlashingConfig::BeagleConnectFreedom {
                img,
                port,
                customization,
            } => {
                tracing::info!("Port opened");
                let img = crate::img::OsImage::from_selected_image(img, &downloader, &chan).await?;
                tracing::info!("Image opened");

                bcf::flash(img, &port, &chan, customization.verify).await
            }
            FlashingConfig::Msp430 { img, port } => {
                let mut img =
                    crate::img::OsImage::from_selected_image(img, &downloader, &chan).await?;
                tracing::info!("Image opened");

                let mut data = String::new();
                img.read_to_string(&mut data)
                    .map_err(|e| crate::img::Error::FailedToReadImage(e.to_string()))?;
                let bin = util::bin_file_from_str(data).map_err(|e| {
                    crate::img::Error::FailedToReadImage(format!("Invalid image format: {e}"))
                })?;

                tokio::task::spawn_blocking(move || msp430::flash(bin, &port, &chan))
                    .await
                    .expect("Tokio runtime failed to spawn blocking task")
            }
            #[cfg(any(feature = "pb2_mspm0_raw", feature = "pb2_mspm0_dbus"))]
            FlashingConfig::Pb2Mspm0 {
                img,
                persist_eeprom,
            } => {
                let mut img =
                    crate::img::OsImage::from_selected_image(img, &downloader, &chan).await?;
                tracing::info!("Image opened");

                let mut data = String::new();
                img.read_to_string(&mut data)
                    .map_err(|e| crate::img::Error::FailedToReadImage(e.to_string()))?;
                let bin = util::bin_file_from_str(data).map_err(|e| {
                    crate::img::Error::FailedToReadImage(format!("Invalid image format: {e}"))
                })?;

                crate::flasher::pb2_mspm0::flash(bin, &chan, persist_eeprom).await
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "config", derive(serde::Deserialize, serde::Serialize))]
pub enum Flasher {
    #[default]
    SdCard,
    BeagleConnectFreedom,
    Msp430Usb,
    #[cfg(any(feature = "pb2_mspm0_raw", feature = "pb2_mspm0_dbus"))]
    Pb2Mspm0,
}

impl Flasher {
    pub async fn destinations(&self) -> HashSet<Destination> {
        match self {
            Flasher::SdCard => tokio::task::block_in_place(sd::destinations),
            Flasher::BeagleConnectFreedom => tokio::task::block_in_place(bcf::possible_devices),
            Flasher::Msp430Usb => tokio::task::block_in_place(msp430::possible_devices),
            #[cfg(any(feature = "pb2_mspm0_raw", feature = "pb2_mspm0_dbus"))]
            Flasher::Pb2Mspm0 => crate::flasher::pb2_mspm0::possible_devices().await,
        }
    }

    pub fn destination_selectable(&self) -> bool {
        match self {
            #[cfg(any(feature = "pb2_mspm0_raw", feature = "pb2_mspm0_dbus"))]
            Self::Pb2Mspm0 => false,
            _ => true,
        }
    }

    pub fn file_filter(&self) -> (&'static str, &'static [&'static str]) {
        match self {
            Flasher::SdCard => ("image", &["img", "xz"]),
            Flasher::BeagleConnectFreedom => ("firmware", &["bin", "hex", "txt", "xz"]),
            Flasher::Msp430Usb => ("firmware", &["hex", "txt", "xz"]),
            #[cfg(any(feature = "pb2_mspm0_raw", feature = "pb2_mspm0_dbus"))]
            Flasher::Pb2Mspm0 => ("firmware", &["hex", "txt", "xz"]),
        }
    }
}
