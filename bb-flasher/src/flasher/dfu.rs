use crate::{BBFlasher, BBFlasherTarget, DownloadFlashingStatus, Resolvable};

use std::borrow::Cow;
use std::collections::HashSet;
use std::io;

use futures::channel::mpsc;

#[derive(Hash, Eq, PartialEq)]
pub struct Target(bb_flasher_dfu::Device);

impl Target {
    async fn destinations_internal() -> HashSet<Self> {
        tokio::task::spawn_blocking(|| bb_flasher_dfu::devices().into_iter().map(Self).collect())
            .await
            .unwrap()
    }

    pub const fn bus_number(&self) -> u8 {
        self.0.bus_num
    }

    pub const fn port_num(&self) -> u8 {
        self.0.port_num
    }

    pub const fn vendor_id(&self) -> u16 {
        self.0.vendor_id
    }

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

    async fn destinations() -> HashSet<Self> {
        Self::destinations_internal().await
    }

    fn identifier(&self) -> Cow<'_, str> {
        Cow::Owned(format!(
            "{:02x}:{:02x}:{:04x}:{:04x}",
            self.0.bus_num, self.0.port_num, self.0.vendor_id, self.0.product_id
        ))
    }
}

pub struct Flasher<R: Resolvable> {
    imgs: Vec<(String, R)>,
    vendor_id: u16,
    product_id: u16,
    bus_num: u8,
    port_num: u8,
    cancel: Option<tokio_util::sync::CancellationToken>,
}

impl<R> Flasher<R>
where
    R: Resolvable,
{
    const fn new(
        imgs: Vec<(String, R)>,
        bus_num: u8,
        port_num: u8,
        vendor_id: u16,
        product_id: u16,
        cancel: Option<tokio_util::sync::CancellationToken>,
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

    pub fn from_identifier(
        imgs: Vec<(String, R)>,
        id: &str,
        cancel: Option<tokio_util::sync::CancellationToken>,
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

impl<R> BBFlasher for Flasher<R>
where
    R: Resolvable<ResolvedType = (crate::OsImage, u64)> + Send + 'static,
{
    async fn flash(self, chan: Option<mpsc::Sender<DownloadFlashingStatus>>) -> anyhow::Result<()> {
        let c = if let Some(mut c) = chan {
            let (tx, mut rx) = tokio::sync::mpsc::channel(2);

            tokio::spawn(async move {
                // Should run until tx is dropped, i.e. flasher task is done.
                // If it is aborted, then cancel should be dropped, thereby signaling the flasher task to abort
                while let Some(x) = rx.recv().await {
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
        .await
        .map_err(Into::into)
    }
}
