use std::{borrow::Cow, fmt::Display, io::Read};
use tokio::sync::mpsc;

use crate::{BBFlasher, BBFlasherTarget};

/// MSPM0 UART target
#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct Target(String);

impl Target {
    pub fn path(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for Target {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl BBFlasherTarget for Target {
    const FILE_TYPES: &[&str] = &["bin", "hex", "txt", "xz"];

    fn destinations(_: bool) -> impl Future<Output = std::collections::HashSet<Self>> {
        let temp = bb_flasher_mspm0::ports().into_iter().map(Self).collect();

        std::future::ready(temp)
    }

    fn identifier(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.0)
    }
}

impl Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Flasher to flash MSPM0 Images
///
/// # Supported Image Formats
///
/// - TI-TXT
/// - iHex
/// - xz: Xz compressed files for any of the above
#[derive(Debug, Clone)]
pub struct Flasher<I> {
    img: I,
    port: String,
    verify: bool,
    cancel: Option<tokio_util::sync::CancellationToken>,
}

impl<I> Flasher<I> {
    pub fn new(
        img: I,
        port: Target,
        verify: bool,
        cancel: Option<tokio_util::sync::CancellationToken>,
    ) -> Self {
        Self {
            img,
            port: port.0,
            verify,
            cancel,
        }
    }
}

impl<I> BBFlasher for Flasher<I>
where
    I: Future<Output = std::io::Result<(crate::OsImage, u64)>>,
{
    async fn flash(
        self,
        chan: Option<mpsc::Sender<crate::DownloadFlashingStatus>>,
    ) -> anyhow::Result<()> {
        let port = self.port;
        let verify = self.verify;
        let img = {
            let (mut img, _) = self
                .img
                .await
                .map_err(|source| crate::common::FlasherError::ImageResolvingError { source })?;

            tokio::task::spawn_blocking(move || {
                let mut data = Vec::new();
                img.read_to_end(&mut data)?;
                Ok::<Vec<u8>, std::io::Error>(data)
            })
            .await
            .unwrap()
            .map_err(|source| crate::common::FlasherError::ImageResolvingError { source })?
        };

        let flasher_task = if let Some(chan) = chan {
            let (tx, mut rx) = tokio::sync::mpsc::channel(20);
            let flasher_task = tokio::task::spawn_blocking(move || {
                bb_flasher_mspm0::flash(&img, &port, verify, Some(tx), self.cancel)
            });

            // Should run until tx is dropped, i.e. flasher task is done.
            // If it is aborted, then cancel should be dropped, thereby signaling the flasher task to abort
            while let Some(x) = rx.recv().await {
                let _ = chan.try_send(x.into());
            }

            flasher_task
        } else {
            tokio::task::spawn_blocking(move || {
                bb_flasher_mspm0::flash(&img, &port, verify, None, self.cancel)
            })
        };

        flasher_task.await.unwrap().map_err(Into::into)
    }
}

impl From<bb_flasher_mspm0::Status> for crate::DownloadFlashingStatus {
    fn from(value: bb_flasher_mspm0::Status) -> Self {
        match value {
            bb_flasher_mspm0::Status::Preparing => Self::Preparing,
            bb_flasher_mspm0::Status::Flashing(x) => Self::FlashingProgress(x),
            bb_flasher_mspm0::Status::Verifying => Self::Verifying,
        }
    }
}
