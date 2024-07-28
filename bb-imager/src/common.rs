//! Stuff common to all the flashers

use std::path::PathBuf;

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
    pub async fn open(&self) -> crate::error::Result<tokio::fs::File> {
        use std::os::fd::{FromRawFd, IntoRawFd};

        let dbus_client = udisks2::Client::new().await?;
        let obj = dbus_client
            .object(self.block.clone().unwrap())
            .unwrap()
            .block()
            .await
            .unwrap();

        let fd = obj.open_device("rw", Default::default()).await.unwrap();

        unsafe {
            Ok(tokio::fs::File::from_raw_fd(
                std::os::fd::OwnedFd::from(fd).into_raw_fd(),
            ))
        }
    }
}

#[derive(Clone, Debug)]
pub struct State {
    #[cfg(target_os = "linux")]
    pub dbus_client: udisks2::Client,
}

impl State {
    pub async fn new() -> crate::error::Result<Self> {
        let dbus_client = udisks2::Client::new().await?;

        Ok(Self { dbus_client })
    }
}
