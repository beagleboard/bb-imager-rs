//! Stuff common to all the flashers

use std::{
    io::{Seek, SeekFrom, Write},
    path::PathBuf,
    time::Duration,
};
use thiserror::Error;
use tokio::io::AsyncSeekExt;
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
            FlashingConfig::LinuxSd(config) => {
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
                    .unwrap()
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
        }
    }
}

#[derive(Clone, Debug)]
pub enum FlashingConfig {
    LinuxSd(FlashingSdLinuxConfig),
    Bcf(FlashingBcfConfig),
}

impl From<crate::config::Flasher> for FlashingConfig {
    fn from(value: crate::config::Flasher) -> Self {
        match value {
            crate::config::Flasher::SdCard => Self::LinuxSd(Default::default()),
            crate::config::Flasher::BeagleConnectFreedom => Self::Bcf(Default::default()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FlashingSdLinuxConfig {
    pub verify: bool,
    pub hostname: Option<String>,
    pub timezone: Option<String>,
    pub keymap: Option<String>,
}

impl FlashingSdLinuxConfig {
    pub fn customize<D: std::io::Write + std::io::Seek + std::io::Read>(
        &self,
        dst: &mut D,
    ) -> crate::error::Result<()> {
        let boot_partition = {
            let mbr = mbrman::MBR::read_from(dst, 512).unwrap();

            let boot_part = mbr.get(1).unwrap();
            assert_eq!(boot_part.sys, 12);
            let start_offset: u64 = (boot_part.starting_lba * mbr.sector_size).into();
            let end_offset: u64 =
                start_offset + u64::from(boot_part.sectors) * u64::from(mbr.sector_size);
            let slice = fscommon::StreamSlice::new(dst, start_offset, end_offset).unwrap();
            let boot_stream = fscommon::BufStream::new(slice);
            fatfs::FileSystem::new(boot_stream, fatfs::FsOptions::new()).unwrap()
        };

        let boot_root = boot_partition.root_dir();

        if self.hostname.is_some() || self.timezone.is_some() || self.keymap.is_some() {
            let mut sysconf = boot_root.create_file("sysconf.txt").unwrap();
            sysconf.seek(SeekFrom::End(0)).unwrap();

            if let Some(h) = &self.hostname {
                sysconf
                    .write_all(format!("hostname={h}\n").as_bytes())
                    .unwrap();
            }

            if let Some(tz) = &self.timezone {
                sysconf
                    .write_all(format!("timezone={tz}\n").as_bytes())
                    .unwrap();
            }

            if let Some(k) = &self.keymap {
                sysconf
                    .write_all(format!("keymap={k}\n").as_bytes())
                    .unwrap();
            }
        }

        Ok(())
    }

    pub fn update_verify(mut self, verify: bool) -> Self {
        self.verify = verify;
        self
    }

    pub fn update_hostname(mut self, hostname: Option<String>) -> Self {
        self.hostname = hostname;
        self
    }

    pub fn update_timezone(mut self, timezone: Option<String>) -> Self {
        self.timezone = timezone;
        self
    }

    pub fn update_keymap(mut self, k: Option<String>) -> Self {
        self.keymap = k;
        self
    }
}

impl Default for FlashingSdLinuxConfig {
    fn default() -> Self {
        Self {
            verify: true,
            hostname: Default::default(),
            timezone: Default::default(),
            keymap: Default::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FlashingBcfConfig {
    pub verify: bool,
}

impl FlashingBcfConfig {
    pub fn update_verify(mut self, verify: bool) -> Self {
        self.verify = verify;
        self
    }
}

impl Default for FlashingBcfConfig {
    fn default() -> Self {
        Self { verify: true }
    }
}
