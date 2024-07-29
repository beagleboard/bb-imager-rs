//! Stuff common to all the flashers

use std::path::PathBuf;
use thiserror::Error;

pub(crate) const BUF_SIZE: usize = 64 * 1024;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to Open Destination")]
    FailedToOpenDestination(String),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum FlashingStatus {
    Preparing,
    Flashing,
    FlashingProgress(f32),
    Verifying,
    VerifyingProgress(f32),
    Finished,
}

#[derive(Debug, PartialEq, Clone)]
pub enum DownloadStatus {
    DownloadingProgress(f32),
    VerifyingProgress(f32),
    Finished(PathBuf),
}

#[derive(Debug, Default, Hash, PartialEq, Eq, Clone)]
pub struct Destination {
    pub name: String,
    pub size: Option<u64>,
    #[cfg(target_os = "linux")]
    pub block: Option<udisks2::zbus::zvariant::OwnedObjectPath>,
}

impl Destination {
    #[cfg(target_os = "linux")]
    pub async fn open(&self, state: &State) -> crate::error::Result<tokio::fs::File> {
        use std::os::fd::{FromRawFd, IntoRawFd};

        let obj = state
            .dbus_client
            .object(
                self.block
                    .clone()
                    .ok_or(Error::FailedToOpenDestination(self.name.clone()))?,
            )
            .unwrap()
            .block()
            .await?;

        let fd = obj.open_device("rw", Default::default()).await?;

        Ok(unsafe { tokio::fs::File::from_raw_fd(std::os::fd::OwnedFd::from(fd).into_raw_fd()) })
    }

    #[cfg(windows)]
    pub async fn open(&self, state: &State) -> crate::error::Result<tokio::fs::File> {
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
