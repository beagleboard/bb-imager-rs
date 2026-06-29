//! Enable flashing [BeagleConnect Freedom] [MSP430] firmware, which serves as USB to UART Bridge
//! in the package.
//!
//! [BeagleConnect Freedom]: https://www.beagleboard.org/boards/beagleconnect-freedom
//! [MSP430]: https://www.ti.com/product/MSP430F5503

use std::sync::mpsc;
use std::{borrow::Cow, ffi::CString, fmt::Display};

use crate::common::{BBFlasherTarget, DownloadFlashingStatus};

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

    fn destinations(filter: bool) -> std::collections::HashSet<Self> {
        bb_flasher_bcf::msp430::devices(filter)
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
pub struct Flasher<I> {
    img: I,
    port: std::ffi::CString,
}

impl<I> Flasher<I> {
    pub fn new(img: I, port: Target) -> Self {
        Self {
            img,
            port: port.raw_path,
        }
    }
}

impl<I> Flasher<I>
where
    I: FnOnce() -> std::io::Result<(crate::img::OsImage, u64)> + Send + 'static,
{
    pub fn flash(
        self,
        chan: Option<mpsc::SyncSender<DownloadFlashingStatus>>,
    ) -> anyhow::Result<()> {
        let dst = self.port;
        let img = crate::common::resolve_img(self.img)?;

        if let Some(chan) = chan {
            std::thread::scope(|s| {
                let (tx, rx) = mpsc::sync_channel::<bb_flasher_bcf::Status>(2);
                s.spawn(move || {
                    while let Ok(x) = rx.recv() {
                        let _ = chan.try_send(x.into());
                    }
                });

                bb_flasher_bcf::msp430::flash(&img, &dst, Some(tx))
            })
        } else {
            bb_flasher_bcf::msp430::flash(&img, &dst, None)
        }
        .map_err(Into::into)
    }
}
