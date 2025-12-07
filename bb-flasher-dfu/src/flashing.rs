use std::time::Duration;

use rusb::UsbContext;

use crate::Result;

pub(crate) const DELAY: Duration = Duration::from_secs(1);
const RETRY: usize = 10;

fn open_usb_dev(
    vendor_id: u16,
    product_id: u16,
    bus_num: u8,
    port_num: u8,
) -> Result<rusb::DeviceHandle<rusb::Context>> {
    // Due to some sort of internal caching, creating a new context each time is the most reliable.
    let ctx = rusb::Context::new().unwrap();

    // First try open_device_with_vid_pid. This should be sufficient if only one usb device with
    // the vendor_id and product_id exists
    {
        let dev = ctx
            .open_device_with_vid_pid(vendor_id, product_id)
            .ok_or(crate::Error::UsbDevNotFound)?;

        if dev.device().bus_number() == bus_num && dev.device().port_number() == port_num {
            return Ok(dev);
        }
    }

    // Iterate the device list
    let all_usb_devs = ctx
        .devices()
        .expect("rusb seems to not be implemented for this platform");
    for dev in all_usb_devs.iter() {
        if dev.bus_number() == bus_num && dev.port_number() == port_num {
            return dev
                .open()
                .map_err(|e| crate::Error::FailedToOpen { source: e.into() });
        }
    }

    Err(crate::Error::UsbDevNotFound)
}

fn open_dfu_dev(
    vendor_id: u16,
    product_id: u16,
    bus_num: u8,
    port_num: u8,
    name: &str,
) -> Result<dfu_libusb::Dfu<rusb::Context>> {
    fn inner(
        dev: rusb::DeviceHandle<rusb::Context>,
        name: &str,
    ) -> Result<Option<dfu_libusb::Dfu<rusb::Context>>, dfu_libusb::Error> {
        let langs = dev.read_languages(DELAY)?;
        let active_desc = dev.device().active_config_descriptor()?;

        for intf in active_desc.interfaces() {
            let descs = intf.descriptors();

            for desc in descs {
                let Ok(intf_str) = dev.read_interface_string(langs[0], &desc, DELAY) else {
                    continue;
                };

                if intf_str == name {
                    let r = dfu_libusb::DfuLibusb::from_usb_device(
                        dev.device(),
                        dev,
                        desc.interface_number(),
                        desc.setting_number(),
                    )?;

                    return Ok(Some(r));
                }
            }
        }

        Ok(None)
    }

    let dev = open_usb_dev(vendor_id, product_id, bus_num, port_num)?;
    match inner(dev, name) {
        Ok(Some(x)) => Ok(x),
        Ok(None) => Err(crate::Error::DfuIntfNotFound),
        Err(e) => Err(crate::Error::FailedToOpen { source: e }),
    }
}

fn open_dfu_dev_with_retry(
    vendor_id: u16,
    product_id: u16,
    bus_num: u8,
    port_num: u8,
    name: &str,
) -> Result<dfu_libusb::Dfu<rusb::Context>> {
    // Do RETRY - 1 here so that we can return proper error
    for _ in 1..RETRY {
        match open_dfu_dev(vendor_id, product_id, bus_num, port_num, name) {
            Err(_) => {
                std::thread::sleep(DELAY);
            }
            Ok(x) => return Ok(x),
        }
    }

    open_dfu_dev(vendor_id, product_id, bus_num, port_num, name)
}

pub(crate) fn dfu_write(
    vendor_id: u16,
    product_id: u16,
    bus_num: u8,
    port_num: u8,
    name: String,
    reader: impl std::io::Read,
    size: u32,
) -> Result<()> {
    let mut dev = open_dfu_dev_with_retry(vendor_id, product_id, bus_num, port_num, &name)?;
    dev.download(reader, size)
        .map_err(|source| crate::Error::DownloadFail { img: name, source })?;
    let _ = dev.usb_reset();
    let _ = dev.detach();

    Ok(())
}
