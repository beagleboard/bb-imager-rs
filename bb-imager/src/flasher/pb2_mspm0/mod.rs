#[cfg(feature = "pb2_mspm0_raw")]
mod raw;
use std::io::Read;

#[cfg(feature = "pb2_mspm0_raw")]
use raw::flash;
#[cfg(feature = "pb2_mspm0_raw")]
pub use raw::possible_devices;

#[cfg(all(feature = "pb2_mspm0_dbus", not(feature = "pb2_mspm0_raw")))]
mod dbus;
#[cfg(all(feature = "pb2_mspm0_dbus", not(feature = "pb2_mspm0_raw")))]
use dbus::flash;
#[cfg(all(feature = "pb2_mspm0_dbus", not(feature = "pb2_mspm0_raw")))]
pub use dbus::possible_devices;

use crate::BBFlasher;

pub struct Pb2Mspm0<I: crate::img::ImageFile> {
    img: I,
    persist_eeprom: bool,
}

impl<I> Pb2Mspm0<I>
where
    I: crate::img::ImageFile,
{
    pub const fn new(img: I, persist_eeprom: bool) -> Self {
        Self {
            img,
            persist_eeprom,
        }
    }
}

impl<I> BBFlasher for Pb2Mspm0<I>
where
    I: crate::img::ImageFile,
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
