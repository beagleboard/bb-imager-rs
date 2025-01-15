use std::{collections::HashSet, path::PathBuf};

use futures::StreamExt;
use thiserror::Error;
use zbus::proxy;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    FlashingError(String),
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

pub async fn possible_devices() -> std::collections::HashSet<crate::Destination> {
    let connection = zbus::Connection::system().await.unwrap();
    let proxy = Pocketbeagle2Mspm0Proxy::new(&connection).await.unwrap();

    let (name, path, _) = proxy.device().await.unwrap();

    HashSet::from([crate::Destination::file(name, PathBuf::from(path))])
}

pub async fn flash(
    img: bin_file::BinFile,
    chan: &tokio::sync::mpsc::Sender<crate::DownloadFlashingStatus>,
    persist_eeprom: bool,
) -> crate::error::Result<()> {
    let connection = zbus::Connection::system().await.unwrap();
    let proxy = Pocketbeagle2Mspm0Proxy::new(&connection).await.unwrap();

    let (_, _, flash_size) = proxy.device().await.unwrap();
    let firmware = img.to_bytes(0..(flash_size as usize), None).unwrap();

    let mut stream = proxy.receive_status().await.unwrap();
    let chan_clone = chan.clone();
    let task = tokio::spawn(async move {
        while let Some(v) = stream.next().await {
            let json = v.message().body().deserialize::<String>().unwrap();
            let status = serde_json::from_str::<FlashingStatus>(&json).unwrap();
            let _ = chan_clone.try_send(status.into());
        }
    });

    proxy.flash(&firmware, persist_eeprom).await.unwrap();

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
