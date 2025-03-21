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

use std::collections::HashSet;
use std::io::Read;

use crate::{BBFlasher, BBFlasherTarget, ImageFile};

/// [PocketBeagle 2] [MSPM0L1105] target
///
/// [PocketBeagle 2]: https://www.beagleboard.org/boards/pocketbeagle-2
/// [MSPM0L1105]: https://www.ti.com/product/MSPM0L1105
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Target {
    name: String,
    path: std::path::PathBuf,
}

impl BBFlasherTarget for Target {
    const FILE_TYPES: &[&str] = &["hex", "txt", "xz"];

    async fn destinations() -> HashSet<Self> {
        let temp = destinations().await;
        HashSet::from([Target {
            name: temp.0,
            path: temp.1,
        }])
    }

    fn is_destination_selectable() -> bool {
        false
    }

    fn path(&self) -> &std::path::Path {
        &self.path
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
pub struct Flasher<I: ImageFile> {
    img: I,
    persist_eeprom: bool,
}

impl<I> Flasher<I>
where
    I: ImageFile,
{
    pub const fn new(img: I, persist_eeprom: bool) -> Self {
        Self {
            img,
            persist_eeprom,
        }
    }
}

impl<I> BBFlasher for Flasher<I>
where
    I: ImageFile,
{
    async fn flash(
        self,
        chan: Option<futures::channel::mpsc::Sender<crate::DownloadFlashingStatus>>,
    ) -> std::io::Result<()> {
        let bin = {
            let mut img = crate::img::OsImage::open(self.img, chan.clone()).await?;

            let mut data = String::new();
            img.read_to_string(&mut data)?;
            data.parse().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid firmware")
            })?
        };

        flash(bin, chan, self.persist_eeprom)
            .await
            .map_err(std::io::Error::other)
    }
}
