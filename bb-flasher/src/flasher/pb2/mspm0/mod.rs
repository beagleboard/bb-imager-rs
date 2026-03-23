//! [PocketBeagle 2] contains an [MSPM0L1105] which normally acts as an ADC + EEPROM. It can be
//! programmed using BSL over I2C.
//!
//! [PocketBeagle 2]: https://www.beagleboard.org/boards/pocketbeagle-2
//! [MSPM0L1105]: https://www.ti.com/product/MSPM0L1105

cfg_if::cfg_if! {
    if #[cfg(feature = "pb2_mspm0")] {
        mod raw;
        use raw::*;
    } else if #[cfg(feature = "pb2_mspm0_dbus")] {
        mod dbus;
        use dbus::*;
    }
}

use std::borrow::Cow;
use std::collections::HashSet;
use std::io::Read;
use tokio::sync::mpsc;

use crate::{BBFlasher, BBFlasherTarget};

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
    async fn destinations(_: bool) -> HashSet<Self> {
        let temp = destinations().await;
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

impl<I> BBFlasher for Flasher<I>
where
    I: Future<Output = std::io::Result<(crate::OsImage, u64)>>,
{
    async fn flash(
        self,
        chan: Option<mpsc::Sender<crate::DownloadFlashingStatus>>,
    ) -> anyhow::Result<()> {
        let bin = {
            let (mut img, _) = self
                .img
                .await
                .map_err(|source| crate::common::FlasherError::ImageResolvingError { source })?;

            tokio::task::spawn_blocking(move || {
                let mut data = String::new();
                img.read_to_string(&mut data)?;
                data.parse().map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid firmware")
                })
            })
            .await
            .unwrap()
            .map_err(|source| crate::common::FlasherError::ImageResolvingError { source })
        }?;

        flash(bin, chan, self.persist_eeprom)
            .await
            .map_err(Into::into)
    }
}
