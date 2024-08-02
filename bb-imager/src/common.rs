//! Stuff common to all the flashers

use std::{path::PathBuf, time::Duration};
use thiserror::Error;

pub(crate) const BUF_SIZE: usize = 64 * 1024;

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
    pub path: PathBuf,
    pub size: Option<u64>,
}

impl Destination {
    pub fn port(name: String) -> Self {
        Self {
            name: name.clone(),
            path: PathBuf::from(name),
            size: None,
        }
    }

    #[cfg(target_os = "linux")]
    pub(crate) const fn sd_card(name: String, size: u64, path: PathBuf) -> Self {
        Self {
            name,
            path,
            size: Some(size),
        }
    }

    pub fn from_path(path: PathBuf) -> Self {
        Self {
            name: path.to_string_lossy().to_string(),
            path,
            size: None,
        }
    }

    pub fn open_port(&self) -> crate::error::Result<Box<dyn serialport::SerialPort>> {
        serialport::new(&self.name, 500000)
            .timeout(Duration::from_millis(500))
            .open()
            .map_err(|_| {
                Error::FailedToOpenDestination(format!("Failed to open serial port {}", self.name))
            })
            .map_err(Into::into)
    }

    #[cfg(target_os = "linux")]
    pub async fn open_file(&self, state: &State) -> crate::error::Result<std::fs::File> {
        use std::{
            collections::HashMap,
            os::fd::{FromRawFd, IntoRawFd},
        };

        let devs = state
            .dbus_client
            .manager()
            .resolve_device(
                HashMap::from([("path", self.path.to_str().unwrap().into())]),
                HashMap::new(),
            )
            .await?;

        let block = devs
            .first()
            .ok_or(Error::FailedToOpenDestination("Not Found".to_owned()))?
            .to_owned();

        let obj = state.dbus_client.object(block).unwrap().block().await?;

        let fd = obj.open_device("rw", Default::default()).await?;

        Ok(unsafe { std::fs::File::from_raw_fd(std::os::fd::OwnedFd::from(fd).into_raw_fd()) })
    }

    #[cfg(windows)]
    pub async fn open(&self, state: &State) -> crate::error::Result<std::fs::File> {
        todo!()
    }
}

#[derive(Clone, Debug)]
pub struct State {
    #[cfg(target_os = "linux")]
    pub dbus_client: udisks2::Client,
}

impl State {
    #[cfg(target_os = "linux")]
    pub async fn new() -> crate::error::Result<Self> {
        let dbus_client = udisks2::Client::new().await?;

        Ok(Self { dbus_client })
    }

    #[cfg(windows)]
    pub async fn new() -> crate::error::Result<Self> {
        Ok(Self {})
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

pub async fn download_and_flash(
    img: SelectedImage,
    dst: Destination,
    flasher: crate::config::Flasher,
    state: State,
    downloader: crate::download::Downloader,
    chan: std::sync::mpsc::Sender<DownloadFlashingStatus>,
) -> crate::error::Result<()> {
    tracing::info!("Preparing...");
    let _ = chan.send(DownloadFlashingStatus::Preparing);

    match flasher {
        crate::config::Flasher::SdCard => {
            let port = dst.open_file(&state).await?;
            let img = crate::img::OsImage::from_selected_image(img, &downloader, &chan).await?;

            tokio::task::block_in_place(move || crate::sd::flash(img, port, &chan))
        }
        crate::config::Flasher::BeagleConnectFreedom => {
            let port = dst.open_port()?;
            let img = crate::img::OsImage::from_selected_image(img, &downloader, &chan).await?;

            tokio::task::block_in_place(move || crate::bcf::flash(img, port, &chan))
        }
    }
}
