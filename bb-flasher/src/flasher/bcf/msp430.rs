//! Enable flashing [BeagleConnect Freedom] [MSP430] firmware, which serves as USB to UART Bridge
//! in the package.
//!
//! [BeagleConnect Freedom]: https://www.beagleboard.org/boards/beagleconnect-freedom
//! [MSP430]: https://www.ti.com/product/MSP430F5503

use std::{ffi::CString, fmt::Display, io::Read};

use futures::StreamExt;

use crate::{BBFlasher, BBFlasherTarget, Resolvable};

/// BeagleConnect Freedom MSP430 target
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Target {
    raw_path: CString,
    display_path: String,
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

    fn is_destination_selectable() -> bool {
        true
    }

    fn path(&self) -> &std::path::Path {
        std::path::Path::new(&self.display_path)
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
    I: Resolvable<ResolvedType = crate::OsImage>,
{
    async fn flash(
        self,
        chan: Option<futures::channel::mpsc::Sender<crate::DownloadFlashingStatus>>,
    ) -> std::io::Result<()> {
        let dst = self.port;
        let img = {
            let mut img = self.img.resolve().await?;
            let mut data = Vec::new();
            img.read_to_end(&mut data)?;
            data
        };

        let flasher_task = if let Some(chan) = chan {
            let (tx, rx) = futures::channel::mpsc::channel(20);
            let flasher_task = tokio::task::spawn_blocking(move || {
                bb_flasher_bcf::msp430::flash(&img, &dst, Some(tx))
            });

            // Should run until tx is dropped, i.e. flasher task is done.
            // If it is aborted, then cancel should be dropped, thereby signaling the flasher task to abort
            let _ = rx.map(Into::into).map(Ok).forward(chan).await;

            flasher_task
        } else {
            tokio::task::spawn_blocking(move || bb_flasher_bcf::msp430::flash(&img, &dst, None))
        };

        flasher_task.await.unwrap().map_err(std::io::Error::other)
    }
}
