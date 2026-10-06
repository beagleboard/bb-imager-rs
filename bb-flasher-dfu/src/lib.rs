//! A library to flash BeagleBoard devices over [USB DFU].
//!
//! Use [`devices`] to find connected USB devices and [`flash`] to download one or more images to
//! the DFU interfaces of a device.
//!
//! [USB DFU]: https://www.usb.org/sites/default/files/DFU_1.1.pdf

mod flashing;
mod helpers;

use std::{io, sync::mpsc};
use thiserror::Error;

use flashing::dfu_write;
use helpers::{check_token, is_dfu_device};

use bb_helper::{cancel::CancellationToken, reader_progress::ReaderWithProgress};

/// Result type for this crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors for this crate
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Failed to open the USB device or its DFU interface.
    #[error("Failed to open device.")]
    FailedToOpen {
        /// Underlying DFU error.
        #[source]
        source: dfu_libusb::Error,
    },
    /// Failed to download an image to the device.
    #[error("Failed to download {img}.")]
    DownloadFail {
        /// Name of the image.
        img: String,
        /// Underlying DFU error.
        #[source]
        source: dfu_libusb::Error,
    },
    /// Unknown error occurred during image resolution.
    #[error("Failed to fetch firmware image.")]
    ImgResolveFail {
        /// Underlying I/O error.
        #[from]
        #[source]
        source: io::Error,
    },
    /// No USB device matches the given vendor/product id and bus/port number.
    #[error("USB device not found.")]
    UsbDevNotFound,
    /// The device has no DFU interface with the given image name.
    #[error("DFU interface not found.")]
    DfuIntfNotFound,
    /// Aborted before completing.
    #[error("Aborted before completing.")]
    Aborted,
    /// Image is larger than the 4 GiB supported by DFU.
    #[error("Image is too large.")]
    ImageTooLarge,
    /// Failed to access USB devices.
    #[error("Failed to access USB devices.")]
    UsbUnavailable {
        /// Underlying USB error.
        #[source]
        source: rusb::Error,
    },
}

/// A connected USB device.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Device {
    /// USB bus number.
    pub bus_num: u8,
    /// USB port number.
    pub port_num: u8,
    /// USB vendor id.
    pub vendor_id: u16,
    /// USB product id.
    pub product_id: u16,
    /// Human readable name: `"<manufacturer>, <product>"`, or `"<vid>:<pid>"` in hex if the
    /// device does not provide these strings.
    pub name: String,
}

/// Returns connected USB devices. If `filter` is `true`, only devices with a DFU interface are
/// returned.
///
/// Devices that cannot be opened are skipped. Returns nothing if USB devices cannot be listed.
pub fn devices(filter: bool) -> Vec<Device> {
    let devs = match rusb::devices() {
        Ok(x) => x,
        Err(e) => {
            tracing::warn!("Failed to list USB devices: {e}");
            return Vec::new();
        }
    };

    devs.iter()
        .filter(|x| if filter { is_dfu_device(x) } else { true })
        .flat_map(|x| match (x.device_descriptor(), x.open()) {
            (Ok(desc), Ok(dev)) => {
                let name = match (
                    dev.read_manufacturer_string_ascii(&desc),
                    dev.read_product_string_ascii(&desc),
                ) {
                    (Ok(m), Ok(p)) => format!("{m}, {p}"),
                    _ => format!("{:04x}:{:04x}", desc.vendor_id(), desc.product_id()),
                };
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

/// Flash images to a USB device over DFU.
///
/// - `imgs`: `(name, opener)` pairs, flashed in order. `name` selects the DFU interface (by its
///   interface string) to download to. `opener` returns a reader for the image and its size.
/// - `vendor_id`, `product_id`, `bus_num`, `port_num`: identify the device, see [`Device`].
/// - `chan`: optional channel for overall progress across all images, in `0.0..=1.0`.
/// - `cancel`: optional token to abort between images.
pub fn flash<R, I>(
    imgs: Vec<(String, R)>,
    vendor_id: u16,
    product_id: u16,
    bus_num: u8,
    port_num: u8,
    chan: Option<mpsc::SyncSender<f32>>,
    cancel: Option<CancellationToken>,
) -> Result<()>
where
    R: FnOnce() -> std::io::Result<(I, u64)>,
    I: io::Read,
{
    let imgs_count = imgs.len();

    for (idx, img) in imgs.into_iter().enumerate() {
        check_token(cancel.as_ref())?;

        let name = img.0.clone();
        let (img_reader, size) = img.1().map_err(|e| Error::ImgResolveFail { source: e })?;
        let dfu_size = u32::try_from(size).map_err(|_| Error::ImageTooLarge)?;

        let res = match chan.clone() {
            Some(c) => std::thread::scope(|s| {
                let (tx, rx) = mpsc::sync_channel::<f32>(1);

                s.spawn(move || {
                    let partition: f32 = 1.0 / (imgs_count as f32);
                    let offset = idx as f32 * partition;

                    while let Ok(x) = rx.recv() {
                        let res = offset + x * partition;
                        let _ = c.try_send(res);
                    }
                });

                dfu_write(
                    vendor_id,
                    product_id,
                    bus_num,
                    port_num,
                    name,
                    ReaderWithProgress::new(img_reader, size, Some(tx)),
                    dfu_size,
                )
            }),
            None => dfu_write(
                vendor_id, product_id, bus_num, port_num, name, img_reader, dfu_size,
            ),
        };

        // tiboot3 must succeed. Later images may cause the device to detach/re-enumerate
        // mid-download, so a DownloadFail is expected and ignored for them.
        match res {
            Err(Error::DownloadFail { .. }) if img.0 != "tiboot3.bin" => {}
            r => r?,
        }

        check_token(cancel.as_ref())?;
        std::thread::sleep(flashing::DELAY);
    }

    Ok(())
}
