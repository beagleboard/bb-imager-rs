//! Stuff common to all the flashers

use std::{
    ffi::CString,
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    time::Duration,
};
use thiserror::Error;
use tokio::io::AsyncSeekExt;
use tokio_serial::SerialPortBuilderExt;

use crate::flasher::{bcf, msp430, sd};

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
    Finished,
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
            FlashingConfig::Msp430 => {
                let mut img = crate::img::OsImage::from_selected_image(
                    self.img,
                    &self.downloader,
                    &self.chan,
                )
                .await?;
                tracing::info!("Image opened");

                let mut data = String::new();
                img.read_to_string(&mut data).unwrap();

                let data: Vec<&str> = data.split_whitespace().collect();

                let mut bin = bin_file::BinFile::new();
                bin.add_ihex(data, true).unwrap();

                tokio::task::spawn_blocking(move || msp430::flash(bin, self.dst, &self.chan))
                    .await
                    .unwrap()
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum FlashingConfig {
    LinuxSd(FlashingSdLinuxConfig),
    Bcf(FlashingBcfConfig),
    Msp430,
}

impl From<crate::config::Flasher> for FlashingConfig {
    fn from(value: crate::config::Flasher) -> Self {
        match value {
            crate::config::Flasher::SdCard => Self::LinuxSd(Default::default()),
            crate::config::Flasher::BeagleConnectFreedom => Self::Bcf(Default::default()),
            crate::config::Flasher::Msp430Usb => Self::Msp430,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FlashingSdLinuxConfig {
    pub verify: bool,
    pub hostname: Option<String>,
    pub timezone: Option<String>,
    pub keymap: Option<String>,
    pub user: Option<(String, String)>,
    pub wifi: Option<(String, String)>,
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

        if self.hostname.is_some()
            || self.timezone.is_some()
            || self.keymap.is_some()
            || self.user.is_some()
            || self.wifi.is_some()
        {
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

            if let Some((u, p)) = &self.user {
                sysconf
                    .write_all(format!("user_name={u}\n").as_bytes())
                    .unwrap();
                sysconf
                    .write_all(format!("user_password={p}\n").as_bytes())
                    .unwrap();
            }

            if let Some((ssid, _)) = &self.wifi {
                sysconf
                    .write_all(format!("iwd_psk_file={ssid}.psk\n").as_bytes())
                    .unwrap();
            }
        }

        if let Some((ssid, psk)) = &self.wifi {
            let mut wifi_file = boot_root
                .create_file(format!("services/{ssid}.psk").as_str())
                .unwrap();

            wifi_file
                .write_all(
                    format!("[Security]\nPassphrase={psk}\n\n[Settings]\nAutoConnect=true")
                        .as_bytes(),
                )
                .unwrap();
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

    pub fn update_user(mut self, v: Option<(String, String)>) -> Self {
        self.user = v;
        self
    }

    pub fn update_wifi(mut self, v: Option<(String, String)>) -> Self {
        self.wifi = v;
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
            user: Default::default(),
            wifi: Default::default(),
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
