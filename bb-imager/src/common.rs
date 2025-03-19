//! Stuff common to all the flashers

use std::{collections::HashSet, ffi::CString, path::PathBuf};

use crate::flasher::{bcf, msp430, sd};

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
        path: PathBuf,
        size: u64,
    },
    HidRaw(std::ffi::CString),
    File(String, PathBuf),
}

impl Destination {
    pub(crate) const fn port(name: String) -> Self {
        Self::Port(name)
    }

    pub const fn size(&self) -> u64 {
        if let Self::SdCard { size, .. } = self {
            *size
        } else {
            0
        }
    }

    pub(crate) const fn sd_card(name: String, size: u64, path: PathBuf) -> Self {
        Self::SdCard { name, path, size }
    }

    pub(crate) const fn hidraw(path: CString) -> Self {
        Self::HidRaw(path)
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

pub trait BBFlasher {
    fn flash(
        self,
        chan: Option<futures::channel::mpsc::Sender<DownloadFlashingStatus>>,
    ) -> impl Future<Output = std::io::Result<()>>;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
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
