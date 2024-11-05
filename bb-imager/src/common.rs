//! Stuff common to all the flashers

use std::{
    ffi::CString,
    io::{Read, SeekFrom},
    path::PathBuf,
    time::Duration,
};
use thiserror::Error;
use tokio::io::AsyncSeekExt;
use tokio_serial::SerialPortBuilderExt;

use crate::{
    flasher::{bcf, msp430, sd},
    util,
};

pub(crate) const BUF_SIZE: usize = 32 * 1024;

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

    pub fn open_port(&self) -> crate::error::Result<tokio_serial::SerialStream> {
        if let Self::Port(path) = self {
            tokio_serial::new(path, 500000)
                .timeout(Duration::from_millis(500))
                .open_native_async()
                .map_err(|_| {
                    Error::FailedToOpenDestination(format!("Failed to open serial port {}", path))
                })
                .map_err(Into::into)
        } else {
            unreachable!()
        }
    }

    pub fn open_hidraw(&self) -> crate::error::Result<hidapi::HidDevice> {
        if let Self::HidRaw(path) = self {
            hidapi::HidApi::new()
                .map_err(|e| Error::FailedToOpenDestination(e.to_string()))?
                .open_path(path)
                .map_err(|e| Error::FailedToOpenDestination(e.to_string()))
                .map_err(Into::into)
        } else {
            unreachable!()
        }
    }
}

impl std::fmt::Display for Destination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Destination::Port(p) => write!(f, "{}", p),
            Destination::SdCard { name, .. } => write!(f, "{}", name),
            Destination::HidRaw(p) => write!(f, "{}", p.to_string_lossy()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SelectedImage {
    Local(PathBuf),
    Remote {
        name: String,
        url: url::Url,
        extract_sha256: [u8; 32],
    },
    /// For cases like formatting where no image needs to be selected.
    Null(&'static str),
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
            SelectedImage::Null(x) => write!(f, "{}", x),
        }
    }
}

impl From<&crate::config::OsList> for SelectedImage {
    fn from(value: &crate::config::OsList) -> Self {
        Self::remote(value.name.clone(), value.url.clone(), value.image_sha256)
    }
}

#[derive(Debug, Clone)]
pub struct Flasher {
    img: SelectedImage,
    dst: crate::Destination,
    downloader: crate::download::Downloader,
    chan: tokio::sync::mpsc::Sender<DownloadFlashingStatus>,
    config: FlashingConfig,
}

impl Flasher {
    pub const fn new(
        img: SelectedImage,
        dst: crate::Destination,
        downloader: crate::download::Downloader,
        chan: tokio::sync::mpsc::Sender<DownloadFlashingStatus>,
        config: FlashingConfig,
    ) -> Self {
        Self {
            img,
            dst,
            downloader,
            chan,
            config,
        }
    }

    pub async fn download_flash_customize(self) -> crate::error::Result<()> {
        match self.config {
            FlashingConfig::LinuxSd(None) => self.dst.format().await,
            FlashingConfig::LinuxSd(Some(config)) => {
                let mut disk = self.dst.open().await?;
                let img = crate::img::OsImage::from_selected_image(
                    self.img,
                    &self.downloader,
                    &self.chan,
                )
                .await?;

                sd::flash(img, &mut disk, &self.chan, config.verify).await?;
                disk.seek(SeekFrom::Start(0)).await?;

                let mut std_disk = disk.into_std().await;

                tokio::task::spawn_blocking(move || config.customize(&mut std_disk))
                    .await
                    .expect("Tokio runtime failed to spawn blocking task")
            }
            FlashingConfig::Bcf(config) => {
                let port = self.dst.open_port()?;
                tracing::info!("Port opened");
                let img = crate::img::OsImage::from_selected_image(
                    self.img,
                    &self.downloader,
                    &self.chan,
                )
                .await?;
                tracing::info!("Image opened");

                bcf::flash(img, port, &self.chan, config.verify).await
            }
            FlashingConfig::Msp430 => {
                let mut img = crate::img::OsImage::from_selected_image(
                    self.img,
                    &self.downloader,
                    &self.chan,
                )
                .await?;
                tracing::info!("Image opened");

                let mut data = String::new();
                img.read_to_string(&mut data)
                    .map_err(|e| crate::img::Error::FailedToReadImage(e.to_string()))?;
                let bin = util::bin_file_from_str(data).map_err(|e| {
                    crate::img::Error::FailedToReadImage(format!("Invalid image format: {e}"))
                })?;

                tokio::task::spawn_blocking(move || msp430::flash(bin, self.dst, &self.chan))
                    .await
                    .expect("Tokio runtime failed to spawn blocking task")
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum FlashingConfig {
    LinuxSd(Option<sd::FlashingSdLinuxConfig>),
    Bcf(bcf::FlashingBcfConfig),
    Msp430,
}

impl FlashingConfig {
    pub fn new(flasher: crate::config::Flasher, image: &SelectedImage) -> Self {
        match flasher {
            crate::config::Flasher::SdCard => match image {
                SelectedImage::Null(_) => Self::LinuxSd(None),
                _ => Self::LinuxSd(Some(Default::default())),
            },
            crate::config::Flasher::BeagleConnectFreedom => Self::Bcf(Default::default()),
            crate::config::Flasher::Msp430Usb => Self::Msp430,
        }
    }
}
