//! Flash devices over [USB DFU].
//!
//! [USB DFU]: https://www.usb.org/sites/default/files/DFU_1.1.pdf

use crate::common::{BBFlasherTarget, DownloadFlashingStatus};

use std::borrow::Cow;
use std::io;
use std::sync::mpsc;

use bb_helper::cancel::CancellationToken;

/// USB DFU device
#[derive(Hash, Eq, PartialEq)]
pub struct Target(bb_flasher_dfu::Device);

impl Target {
    fn destinations_internal(filter: bool) -> impl Iterator<Item = Self> {
        bb_flasher_dfu::devices(filter).into_iter().map(Self)
    }

    /// USB bus number.
    pub const fn bus_number(&self) -> u8 {
        self.0.bus_num
    }

    /// USB port number.
    pub const fn port_num(&self) -> u8 {
        self.0.port_num
    }

    /// USB vendor id.
    pub const fn vendor_id(&self) -> u16 {
        self.0.vendor_id
    }

    /// USB product id.
    pub const fn product_id(&self) -> u16 {
        self.0.product_id
    }
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.0.name)
    }
}

impl BBFlasherTarget for Target {
    const FILE_TYPES: &[&str] = &[];

    fn destinations(filter: bool) -> impl Iterator<Item = Self> {
        Self::destinations_internal(filter)
    }

    fn identifier(&self) -> Cow<'_, str> {
        Cow::Owned(format!(
            "{:02x}:{:02x}:{:04x}:{:04x}",
            self.0.bus_num, self.0.port_num, self.0.vendor_id, self.0.product_id
        ))
    }
}

/// Flasher to download one or more images to a USB DFU device.
pub struct Flasher<R> {
    imgs: Vec<(String, R)>,
    vendor_id: u16,
    product_id: u16,
    bus_num: u8,
    port_num: u8,
    cancel: Option<CancellationToken>,
}

impl<R> Flasher<R> {
    const fn new(
        imgs: Vec<(String, R)>,
        bus_num: u8,
        port_num: u8,
        vendor_id: u16,
        product_id: u16,
        cancel: Option<CancellationToken>,
    ) -> Self {
        Self {
            imgs,
            vendor_id,
            product_id,
            bus_num,
            port_num,
            cancel,
        }
    }

    /// Create a flasher for a device given its [`BBFlasherTarget::identifier`]
    /// (`bus:port:vendor_id:product_id` in hex).
    ///
    /// `imgs` are `(name, opener)` pairs, flashed in order. `name` selects the DFU interface to
    /// download to.
    pub fn from_identifier(
        imgs: Vec<(String, R)>,
        id: &str,
        cancel: Option<CancellationToken>,
    ) -> io::Result<Self> {
        let ids = id.split(":").map(|x| x.trim()).collect::<Vec<_>>();
        if ids.len() != 4 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid identifier",
            ));
        }

        let bus_num = u8::from_str_radix(ids[0], 16)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid bus number"))?;
        let port_num = u8::from_str_radix(ids[1], 16)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid address"))?;
        let vendor_id = u16::from_str_radix(ids[2], 16)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid Vendor ID"))?;
        let product_id = u16::from_str_radix(ids[3], 16)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid Product ID"))?;

        Ok(Self::new(
            imgs, bus_num, port_num, vendor_id, product_id, cancel,
        ))
    }
}

impl<R> Flasher<R>
where
    R: FnOnce() -> std::io::Result<(crate::img::OsImage, u64)>,
{
    /// Flash the images. Progress across all images is reported over `chan`.
    pub fn flash(
        self,
        chan: Option<mpsc::SyncSender<DownloadFlashingStatus>>,
    ) -> anyhow::Result<()> {
        std::thread::scope(|s| {
            let c = if let Some(c) = chan {
                let (tx, rx) = mpsc::sync_channel(2);

                s.spawn(move || {
                    // Should run until tx is dropped, i.e. flasher task is done.
                    // If it is aborted, then cancel should be dropped, thereby signaling the flasher task to abort
                    while let Ok(x) = rx.recv() {
                        let _ = c.try_send(DownloadFlashingStatus::FlashingProgress(x));
                    }
                });

                Some(tx)
            } else {
                None
            };

            bb_flasher_dfu::flash(
                self.imgs,
                self.vendor_id,
                self.product_id,
                self.bus_num,
                self.port_num,
                c,
                self.cancel,
            )
            .map_err(Into::into)
        })
    }
}
