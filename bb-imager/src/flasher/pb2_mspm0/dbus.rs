use std::{collections::HashSet, path::PathBuf};

use futures::StreamExt;
use thiserror::Error;
use zbus::proxy;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    FlashingError(String),
    #[error("{0}")]
    ZbusError(zbus::Error),
    #[error("Image is not valid")]
    InvalidImage,
}

impl From<zbus::Error> for Error {
    fn from(value: zbus::Error) -> Self {
        Self::ZbusError(value)
    }
}

#[derive(serde::Deserialize, Debug)]
pub enum FlashingStatus {
    Preparing,
    Flashing(f32),
    Verifying,
}

#[proxy(
    interface = "org.beagleboard.ImagingService.Pocketbeagle2Mspm0",
    default_service = "org.beagleboard.ImagingService",
    default_path = "/org/beagleboard/ImagingService/Pocketbeagle2Mspm0"
)]
pub trait Pocketbeagle2Mspm0 {
    /// Check method
    fn check(&self) -> zbus::Result<()>;

    /// Device method
    fn device(&self) -> zbus::Result<(String, String, u64)>;

    /// Flash method
    fn flash(&self, firmware: &[u8], persist_eeprom: bool) -> zbus::Result<()>;

    /// Status signal
    #[zbus(signal)]
    fn status(&self, message: &str) -> zbus::Result<()>;
}

pub async fn possible_devices() -> HashSet<crate::Destination> {
    if let Ok(connection) = zbus::Connection::system().await {
        if let Ok(proxy) = Pocketbeagle2Mspm0Proxy::new(&connection).await {
            if let Ok((name, path, _)) = proxy.device().await {
                return HashSet::from([crate::Destination::file(name, PathBuf::from(path))]);
            }
        }
    }

    tracing::error!("Maybe bb-imager-service is not installed");

    HashSet::new()
}

pub async fn flash(
    img: bin_file::BinFile,
    chan: &tokio::sync::mpsc::Sender<crate::DownloadFlashingStatus>,
    persist_eeprom: bool,
) -> crate::error::Result<()> {
    let connection = zbus::Connection::system().await.map_err(Error::from)?;
    let proxy = Pocketbeagle2Mspm0Proxy::new(&connection)
        .await
        .map_err(Error::from)?;

    let (_, _, flash_size) = proxy.device().await.map_err(Error::from)?;
    let firmware = img
        .to_bytes(0..(flash_size as usize), None)
        .map_err(|_| Error::InvalidImage)?;

    let mut stream = proxy.receive_status().await.map_err(Error::from)?;
    let chan_clone = chan.clone();
    let task = tokio::spawn(async move {
        while let Some(v) = stream.next().await {
            if let Ok(json) = v.message().body().deserialize::<String>() {
                if let Ok(status) = serde_json::from_str::<FlashingStatus>(&json) {
                    let _ = chan_clone.try_send(status.into());
                }
            }
        }
    });

    proxy
        .flash(&firmware, persist_eeprom)
        .await
        .map_err(Error::from)?;

    task.abort();

    Ok(())
}

impl From<FlashingStatus> for crate::DownloadFlashingStatus {
    fn from(value: FlashingStatus) -> Self {
        match value {
            FlashingStatus::Preparing => Self::Preparing,
            FlashingStatus::Flashing(x) => Self::FlashingProgress(x),
            FlashingStatus::Verifying => Self::Verifying,
        }
    }
}
