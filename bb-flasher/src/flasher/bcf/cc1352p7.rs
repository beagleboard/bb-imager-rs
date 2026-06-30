//! Enable flashing [BeagleConnect Freedom] [CC1352P7] firmware, which is the main processor.
//!
//! [BeagleConnect Freedom]: https://www.beagleboard.org/boards/beagleconnect-freedom
//! [CC1352P7]: https://www.ti.com/product/CC1352P7

use std::sync::mpsc;
use std::{borrow::Cow, fmt::Display};

use bb_helper::cancel::CancellationToken;

use crate::common::{BBFlasherTarget, DownloadFlashingStatus};

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

    fn destinations(filter: bool) -> std::collections::HashSet<Self> {
        bb_flasher_bcf::cc1352p7::ports(filter)
            .into_iter()
            .map(Self)
            .collect()
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
pub struct Flasher<I> {
    img: I,
    port: String,
    verify: bool,
    cancel: Option<CancellationToken>,
}

impl<I> Flasher<I> {
    pub fn new(img: I, port: Target, verify: bool, cancel: Option<CancellationToken>) -> Self {
        Self {
            img,
            port: port.0,
            verify,
            cancel,
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
        let port = self.port;
        let verify = self.verify;
        let img = crate::common::resolve_img(self.img)?;

        if let Some(chan) = chan {
            std::thread::scope(|s| {
                let (tx, rx) = mpsc::sync_channel::<bb_flasher_bcf::Status>(2);
                s.spawn(move || {
                    while let Ok(x) = rx.recv() {
                        let _ = chan.try_send(x.into());
                    }
                });

                bb_flasher_bcf::cc1352p7::flash(&img, &port, verify, Some(tx), self.cancel)
            })
        } else {
            bb_flasher_bcf::cc1352p7::flash(&img, &port, verify, None, self.cancel)
        }
        .map_err(Into::into)
    }
}
