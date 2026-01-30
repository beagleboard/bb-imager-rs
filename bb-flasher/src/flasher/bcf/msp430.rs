//! Enable flashing [BeagleConnect Freedom] [MSP430] firmware, which serves as USB to UART Bridge
//! in the package.
//!
//! [BeagleConnect Freedom]: https://www.beagleboard.org/boards/beagleconnect-freedom
//! [MSP430]: https://www.ti.com/product/MSP430F5503

use std::{borrow::Cow, ffi::CString, fmt::Display, io::Read};

use crate::{BBFlasher, BBFlasherTarget, Resolvable};

/// BeagleConnect Freedom MSP430 target
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Target {
    raw_path: CString,
    display_path: String,
}

impl Target {
    pub fn path(&self) -> &str {
        self.display_path.as_str()
    }
}

impl Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.display_path.fmt(f)
    }
}

impl From<String> for Target {
    fn from(value: String) -> Self {
        Self {
            raw_path: CString::new(value.clone()).unwrap(),
            display_path: value,
        }
    }
}

impl BBFlasherTarget for Target {
    const FILE_TYPES: &[&str] = &["hex", "txt", "xz"];

    async fn destinations() -> std::collections::HashSet<Self> {
        bb_flasher_bcf::msp430::devices()
            .into_iter()
            .map(|x| Self {
                display_path: x.to_string_lossy().to_string(),
                raw_path: x,
            })
            .collect()
    }

    fn identifier(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.display_path)
    }
}

/// Flasher to flash [BeagleConnect Freedom] [MSP430] Images
///
/// # Supported Image Formats
///
/// - Ti-TXT
/// - iHex
/// - bin: Raw bins of 704k size
/// - xz: Xz compressed files for any of the above
///
/// [BeagleConnect Freedom]: https://www.beagleboard.org/boards/beagleconnect-freedom
/// [MSP430]: https://www.ti.com/product/MSP430F5503
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Flasher<I: Resolvable> {
    img: I,
    port: std::ffi::CString,
}

impl<I> Flasher<I>
where
    I: Resolvable,
{
    pub fn new(img: I, port: Target) -> Self {
        Self {
            img,
            port: port.raw_path,
        }
    }
}

impl<I> BBFlasher for Flasher<I>
where
    I: Resolvable<ResolvedType = (crate::OsImage, u64)>,
{
    async fn flash(
        self,
        chan: Option<futures::channel::mpsc::Sender<crate::DownloadFlashingStatus>>,
    ) -> anyhow::Result<()> {
        let dst = self.port;
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
                bb_flasher_bcf::msp430::flash(&img, &dst, Some(tx))
            });

            // Should run until tx is dropped, i.e. flasher task is done.
            // If it is aborted, then cancel should be dropped, thereby signaling the flasher task to abort
            while let Some(x) = rx.recv().await {
                let _ = chan.try_send(x.into());
            }

            flasher_task
        } else {
            tokio::task::spawn_blocking(move || bb_flasher_bcf::msp430::flash(&img, &dst, None))
        };

        flasher_task.await.unwrap().map_err(Into::into)
    }
}
