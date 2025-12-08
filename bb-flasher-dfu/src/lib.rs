mod flashing;
mod helpers;

use std::{collections::HashSet, io};
use thiserror::Error;
use tokio::sync::mpsc;

use flashing::dfu_write;
use helpers::{check_token, is_dfu_device};

use crate::helpers::ReaderWithProgress;

pub(crate) type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to open device.")]
    FailedToOpen {
        #[source]
        source: dfu_libusb::Error,
    },
    #[error("Failed to download {img}.")]
    DownloadFail {
        img: String,
        #[source]
        source: dfu_libusb::Error,
    },
    /// Unknown error occured during image resolution.
    #[error("Failed to fetch firmware image.")]
    ImgResolveFail {
        #[from]
        #[source]
        source: io::Error,
    },
    #[error("USB device not found.")]
    UsbDevNotFound,
    #[error("DFU interface not found.")]
    DfuIntfNotFound,
    #[error("Aborted before completing.")]
    Aborted,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Device {
    pub bus_num: u8,
    pub port_num: u8,
    pub vendor_id: u16,
    pub product_id: u16,
    pub name: String,
}

pub fn devices() -> HashSet<Device> {
    rusb::devices()
        .expect("rusb seems to not be implemented for this platform")
        .iter()
        .filter(is_dfu_device)
        .flat_map(|x| match (x.device_descriptor(), x.open()) {
            (Ok(desc), Ok(dev)) => {
                let name = format!(
                    "{}, {}",
                    dev.read_manufacturer_string_ascii(&desc).unwrap(),
                    dev.read_product_string_ascii(&desc).unwrap()
                );
                Some(Device {
                    bus_num: x.bus_number(),
                    port_num: x.port_number(),
                    vendor_id: desc.vendor_id(),
                    product_id: desc.product_id(),
                    name,
                })
            }
            _ => None,
        })
        .collect()
}

pub async fn flash<R, I>(
    imgs: Vec<(String, R)>,
    vendor_id: u16,
    product_id: u16,
    bus_num: u8,
    port_num: u8,
    chan: Option<mpsc::Sender<f32>>,
    cancel: Option<tokio_util::sync::CancellationToken>,
) -> Result<()>
where
    R: bb_helper::resolvable::Resolvable<ResolvedType = (I, u64)>,
    I: io::Read + Send + 'static,
{
    let mut tasks = tokio::task::JoinSet::new();
    let imgs_count = imgs.len();

    for (idx, img) in imgs.into_iter().enumerate() {
        check_token(cancel.as_ref())?;

        let name = img.0.clone();
        let (img_reader, size) = img
            .1
            .resolve(&mut tasks)
            .await
            .map_err(|e| Error::ImgResolveFail { source: e })?;

        let res = match chan.clone() {
            Some(c) => {
                let (tx, mut rx) = tokio::sync::mpsc::channel::<f32>(1);

                tasks.spawn(async move {
                    let partition: f32 = 1.0 / (imgs_count as f32);
                    let offset = idx as f32 * partition;

                    while let Some(x) = rx.recv().await {
                        let res = offset + x * partition;
                        let _ = c.try_send(res);
                    }

                    Ok(())
                });

                tokio::task::spawn_blocking(move || {
                    dfu_write(
                        vendor_id,
                        product_id,
                        bus_num,
                        port_num,
                        name,
                        ReaderWithProgress::new(img_reader, size, tx),
                        size.try_into().unwrap(),
                    )
                })
                .await
                .unwrap()
            }
            None => tokio::task::spawn_blocking(move || {
                dfu_write(
                    vendor_id,
                    product_id,
                    bus_num,
                    port_num,
                    name,
                    img_reader,
                    size.try_into().unwrap(),
                )
            })
            .await
            .unwrap(),
        };

        // For some reason tiboot3 does not exit properly. So need to ignore errors.
        if img.0.as_str() != "tiboot3.bin" {
            match res {
                Err(Error::DownloadFail { .. }) => {}
                _ => return res,
            }
        } else {
            res?;
        }

        check_token(cancel.as_ref())?;
        std::thread::sleep(flashing::DELAY);
    }

    Ok(())
}
