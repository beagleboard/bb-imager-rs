//! Stuff common to all the flashers

use std::{path::PathBuf, time::Duration};
use thiserror::Error;
use tokio_serial::SerialPortBuilderExt;

use crate::flasher::{bcf, sd};

pub(crate) const BUF_SIZE: usize = 32 * 1024;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to Open Destination")]
    FailedToOpenDestination(String),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DownloadFlashingStatus {
    Preparing,
    DownloadingProgress(f32),
    FlashingProgress(f32),
    Verifying,
    VerifyingProgress(f32),
    Finished,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Destination {
    pub name: String,
    pub path: String,
    pub size: Option<u64>,
}

impl Destination {
    pub fn port(name: String) -> Self {
        Self {
            name: name.clone(),
            path: name,
            size: None,
        }
    }

    pub(crate) const fn sd_card(name: String, size: u64, path: String) -> Self {
        Self {
            name,
            path,
            size: Some(size),
        }
    }

    pub fn from_path(path: String) -> Self {
        Self {
            name: path.clone(),
            path,
            size: None,
        }
    }

    pub fn open_port(&self) -> crate::error::Result<tokio_serial::SerialStream> {
        tokio_serial::new(&self.name, 500000)
            .timeout(Duration::from_millis(500))
            .open_native_async()
            .map_err(|_| {
                Error::FailedToOpenDestination(format!("Failed to open serial port {}", self.name))
            })
            .map_err(Into::into)
    }
}

#[derive(Debug, Clone)]
pub enum SelectedImage {
    Local(PathBuf),
    Remote {
        name: String,
        url: url::Url,
        extract_sha256: [u8; 32],
        extract_path: Option<String>,
    },
}

impl SelectedImage {
    pub const fn local(name: PathBuf) -> Self {
        Self::Local(name)
    }

    pub const fn remote(
        name: String,
        url: url::Url,
        download_sha256: [u8; 32],
        extract_path: Option<String>,
    ) -> Self {
        Self::Remote {
            name,
            url,
            extract_sha256: download_sha256,
            extract_path,
        }
    }
}

impl std::fmt::Display for SelectedImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectedImage::Local(p) => write!(f, "{}", p.file_name().unwrap().to_string_lossy()),
            SelectedImage::Remote { name, .. } => write!(f, "{}", name),
        }
    }
}

impl From<&crate::config::OsList> for SelectedImage {
    fn from(value: &crate::config::OsList) -> Self {
        Self::remote(
            value.name.clone(),
            value.url.clone(),
            value.extract_sha256,
            value.extract_path.clone(),
        )
    }
}

pub async fn download_and_flash(
    img: SelectedImage,
    dst: Destination,
    flasher: crate::config::Flasher,
    downloader: crate::download::Downloader,
    chan: tokio::sync::mpsc::Sender<DownloadFlashingStatus>,
    config: FlashingConfig,
) -> crate::error::Result<()> {
    tracing::info!("Preparing...");
    let _ = chan.try_send(DownloadFlashingStatus::Preparing);

    match flasher {
        crate::config::Flasher::SdCard => {
            let port = dst.open().await?;
            let img = crate::img::OsImage::from_selected_image(img, &downloader, &chan).await?;

            sd::flash(img, port, &chan, config.verify).await
        }
        crate::config::Flasher::BeagleConnectFreedom => {
            let port = dst.open_port()?;
            tracing::info!("Port opened");
            let img = crate::img::OsImage::from_selected_image(img, &downloader, &chan).await?;
            tracing::info!("Image opened");

            bcf::flash(img, port, &chan, config.verify).await
        }
    }
}

#[derive(Clone, Debug)]
pub struct FlashingConfig {
    pub verify: bool,
}

impl FlashingConfig {
    pub fn update_verify(mut self, val: bool) -> Self {
        self.verify = val;
        self
    }
}

impl Default for FlashingConfig {
    fn default() -> Self {
        Self { verify: true }
    }
}
