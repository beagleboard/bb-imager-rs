mod flashing;
mod helpers;

use std::{collections::HashSet, io};
use tokio::sync::mpsc;

use flashing::dfu_write;
use helpers::{check_token, dfu_to_io_error, is_dfu_device};

use crate::helpers::ReaderWithProgress;

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
) -> io::Result<()>
where
    R: bb_helper::resolvable::Resolvable<ResolvedType = (I, u64)>,
    I: io::Read + Send + 'static,
{
    let mut tasks = tokio::task::JoinSet::new();
    let imgs_count = imgs.len();

    for (idx, img) in imgs.into_iter().enumerate() {
        check_token(cancel.as_ref())?;

        let name = img.0.clone();
        let (img_reader, size) = img.1.resolve(&mut tasks).await?;

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
                Err(dfu_libusb::Error::Io(e)) if e.kind() == io::ErrorKind::NotFound => {
                    return Err(e);
                }
                _ => {}
            }
        } else {
            res.map_err(dfu_to_io_error)?;
        }

        check_token(cancel.as_ref())?;
        std::thread::sleep(flashing::DELAY);
    }

    Ok(())
}
