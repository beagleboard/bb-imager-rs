use futures::{StreamExt, channel::mpsc};
use thiserror::Error;
use zbus::proxy;

#[derive(Error, Debug)]
pub(crate) enum Error {
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
pub(crate) enum FlashingStatus {
    Preparing,
    Flashing(f32),
    Verifying,
}

#[proxy(
    interface = "org.beagleboard.ImagingService.Pocketbeagle2Mspm0v1",
    default_service = "org.beagleboard.ImagingService",
    default_path = "/org/beagleboard/ImagingService/Pocketbeagle2Mspm0v1"
)]
pub(crate) trait Pocketbeagle2Mspm0 {
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

pub(crate) async fn destinations() -> (String, String) {
    if let Ok(connection) = zbus::Connection::system().await {
        if let Ok(proxy) = Pocketbeagle2Mspm0Proxy::new(&connection).await {
            if let Ok((name, path, _)) = proxy.device().await {
                return (name, path);
            }
        }
    }

    panic!("Maybe bb-imager-service is not installed");
}

pub(crate) async fn flash(
    img: bin_file::BinFile,
    chan: Option<mpsc::Sender<crate::DownloadFlashingStatus>>,
    persist_eeprom: bool,
) -> Result<(), Error> {
    let connection = zbus::Connection::system().await?;
    let proxy = Pocketbeagle2Mspm0Proxy::new(&connection)
        .await
        .map_err(Error::from)?;

    proxy.check().await?;

    let (_, _, flash_size) = proxy.device().await?;
    let firmware = img
        .to_bytes(0..(flash_size as usize), None)
        .map_err(|_| Error::InvalidImage)?;

    let proxy_clone = proxy.clone();
    let progress_task = tokio::spawn(async move {
        if let Some(mut chan) = chan {
            let mut stream = proxy_clone.receive_status().await.unwrap();
            while let Some(v) = stream.next().await {
                if let Ok(json) = v.message().body().deserialize::<String>() {
                    if let Ok(status) = serde_json::from_str::<FlashingStatus>(&json) {
                        let _ = chan.try_send(status.into());
                    }
                }
            }
        }
    });

    proxy.flash(&firmware, persist_eeprom).await?;

    progress_task.abort();

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
