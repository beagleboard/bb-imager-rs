//! [PocketBeagle 2] contains an [MSPM0L1105] which normally acts as an ADC + EEPROM. It can be
//! programmed using BSL over I2C.
//!
//! [PocketBeagle 2]: https://www.beagleboard.org/boards/pocketbeagle-2
//! [MSPM0L1105]: https://www.ti.com/product/MSPM0L1105

mod raw;
use raw::*;

use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::mpsc;

use crate::common::{DownloadFlashingStatus, BBFlasherTarget};

/// [PocketBeagle 2] [MSPM0L1105] target
///
/// [PocketBeagle 2]: https://www.beagleboard.org/boards/pocketbeagle-2
/// [MSPM0L1105]: https://www.ti.com/product/MSPM0L1105
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Target {
    name: String,
    path: String,
}

impl BBFlasherTarget for Target {
    const FILE_TYPES: &[&str] = &["hex", "txt", "xz"];
    const IS_DESTINATION_SELECTABLE: bool = false;

    // Since only a single destination is possible, no need for filters
    fn destinations(_: bool) -> HashSet<Self> {
        let temp = destinations();
        HashSet::from([Target {
            name: temp.0,
            path: temp.1,
        }])
    }

    fn identifier(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.path)
    }
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name.fmt(f)
    }
}

/// Flasher for [MSPM0L1105] in [PocketBeagle 2]
///
/// [PocketBeagle 2]: https://www.beagleboard.org/boards/pocketbeagle-2
/// [MSPM0L1105]: https://www.ti.com/product/MSPM0L1105
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Flasher<I> {
    img: I,
    persist_eeprom: bool,
}

impl<I> Flasher<I> {
    pub const fn new(img: I, persist_eeprom: bool) -> Self {
        Self {
            img,
            persist_eeprom,
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
        let img = self.img;
        let img = crate::common::resolve_img(img)?;
        let img = String::from_utf8(img).map_err(|_| {
            crate::common::FlasherError::ImageResolvingError {
                source: std::io::Error::other("Expected utf8"),
            }
        })?;
        let bin = img
            .parse()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid firmware"))
            .map_err(|source| crate::common::FlasherError::ImageResolvingError { source })?;

        flash(bin, chan, self.persist_eeprom).map_err(Into::into)
    }
}
