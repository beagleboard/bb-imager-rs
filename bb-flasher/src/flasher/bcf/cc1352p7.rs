//! Enable flashing [BeagleConnect Freedom] [CC1352P7] firmware, which is the main processor.
//!
//! [BeagleConnect Freedom]: https://www.beagleboard.org/boards/beagleconnect-freedom
//! [CC1352P7]: https://www.ti.com/product/CC1352P7

use std::{borrow::Cow, fmt::Display, io::Read};

use crate::{BBFlasher, BBFlasherTarget, Resolvable};

/// BeagleConnect Freedom target
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

    fn destinations() -> impl Future<Output = std::collections::HashSet<Self>> {
        let temp = bb_flasher_bcf::cc1352p7::ports()
            .into_iter()
            .map(Self)
            .collect();

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

/// Flasher to flash BeagleConnect Freedom Images
///
/// # Supported Image Formats
///
/// - Ti-TXT
/// - iHex
/// - xz: Xz compressed files for any of the above
#[derive(Debug, Clone)]
pub struct Flasher<I: Resolvable> {
    img: I,
    port: String,
    verify: bool,
    cancel: Option<tokio_util::sync::CancellationToken>,
}

impl<I> Flasher<I>
where
    I: Resolvable,
{
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
    I: Resolvable<ResolvedType = (crate::OsImage, u64)> + Sync,
{
    async fn flash(
        self,
        chan: Option<futures::channel::mpsc::Sender<crate::DownloadFlashingStatus>>,
    ) -> anyhow::Result<()> {
        let port = self.port;
        let verify = self.verify;
        let img = {
            let mut tasks = tokio::task::JoinSet::new();
            let (mut img, _) =
                self.img.resolve(&mut tasks).await.map_err(|source| {
                    crate::common::FlasherError::ImageResolvingError { source }
                })?;

            let resp = tokio::task::spawn_blocking(move || {
                let mut data = Vec::new();
                img.read_to_end(&mut data)?;
                Ok::<Vec<u8>, std::io::Error>(data)
            })
            .await
            .unwrap()
            .map_err(|source| crate::common::FlasherError::ImageResolvingError { source })?;

            while let Some(t) = tasks.join_next().await {
                if let Err(e) = t.unwrap() {
                    tasks.abort_all();
                    return Err(e.into());
                }
            }

            resp
        };

        let flasher_task = if let Some(mut chan) = chan {
            let (tx, mut rx) = tokio::sync::mpsc::channel(20);
            let flasher_task = tokio::task::spawn_blocking(move || {
                bb_flasher_bcf::cc1352p7::flash(&img, &port, verify, Some(tx), self.cancel)
            });

            // Should run until tx is dropped, i.e. flasher task is done.
            // If it is aborted, then cancel should be dropped, thereby signaling the flasher task to abort
            while let Some(x) = rx.recv().await {
                let _ = chan.try_send(x.into());
            }

            flasher_task
        } else {
            tokio::task::spawn_blocking(move || {
                bb_flasher_bcf::cc1352p7::flash(&img, &port, verify, None, self.cancel)
            })
        };

        flasher_task.await.unwrap().map_err(Into::into)
    }
}
