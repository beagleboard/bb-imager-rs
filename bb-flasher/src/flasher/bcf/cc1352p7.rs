//! Enable flashing [BeagleConnect Freedom] [CC1352P7] firmware, which is the main processor.
//!
//! [BeagleConnect Freedom]: https://www.beagleboard.org/boards/beagleconnect-freedom
//! [CC1352P7]: https://www.ti.com/product/CC1352P7

use std::{fmt::Display, io::Read, sync::Arc};

use crate::{BBFlasher, BBFlasherTarget, Resolvable};
use bb_flasher_bcf::cc1352p7::Error;
use futures::StreamExt;

/// BeagleConnect Freedom target
#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct Target(String);

impl From<String> for Target {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl BBFlasherTarget for Target {
    const FILE_TYPES: &[&str] = &["bin", "hex", "txt", "xz"];

    fn destinations() -> impl Future<Output = std::collections::HashSet<Self>> {
        let temp = bb_flasher_bcf::cc1352p7::ports()
            .into_iter()
            .map(Self)
            .collect();

        std::future::ready(temp)
    }

    fn is_destination_selectable() -> bool {
        true
    }

    fn path(&self) -> &std::path::Path {
        std::path::Path::new(&self.0)
    }
}

impl Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Flasher to flash BeagleConnect Freedom Images
///
/// # Supported Image Formats
///
/// - Ti-TXT
/// - iHex
/// - xz: Xz compressed files for any of the above
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Flasher<I: Resolvable> {
    img: I,
    port: String,
    verify: bool,
}

impl<I> Flasher<I>
where
    I: Resolvable,
{
    pub fn new(img: I, port: Target, verify: bool) -> Self {
        Self {
            img,
            port: port.0,
            verify,
        }
    }
}

impl<I> BBFlasher for Flasher<I>
where
    I: Resolvable<ResolvedType = crate::OsImage> + Sync,
{
    async fn flash(
        self,
        chan: Option<futures::channel::mpsc::Sender<crate::DownloadFlashingStatus>>,
    ) -> std::io::Result<()> {
        let cancle = Arc::new(());

        let cancel_weak = Arc::downgrade(&cancle);
        let port = self.port;
        let verify = self.verify;
        let img = {
            let mut img = self.img.resolve().await?;
            let mut data = Vec::new();
            img.read_to_end(&mut data)?;
            data
        };

        let flasher_task = if let Some(chan) = chan {
            let (tx, rx) = futures::channel::mpsc::channel(20);
            let flasher_task = tokio::task::spawn_blocking(move || {
                bb_flasher_bcf::cc1352p7::flash(&img, &port, verify, Some(tx), Some(cancel_weak))
            });

            // Should run until tx is dropped, i.e. flasher task is done.
            // If it is aborted, then cancel should be dropped, thereby signaling the flasher task to abort
            let _ = rx.map(Into::into).map(Ok).forward(chan).await;

            flasher_task
        } else {
            tokio::task::spawn_blocking(move || {
                bb_flasher_bcf::cc1352p7::flash(&img, &port, verify, None, Some(cancel_weak))
            })
        };

        flasher_task.await.unwrap().map_err(|e| match e {
            Error::IoError(error) => error,
            _ => std::io::Error::other(e),
        })
    }
}
