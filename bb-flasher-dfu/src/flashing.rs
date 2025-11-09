use std::{io, time::Duration};

use rusb::UsbContext;

pub(crate) const DELAY: Duration = Duration::from_secs(1);
const RETRY: usize = 10;

fn usb_dev_not_found() -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, "USB Device not found")
}

fn dfu_intf_not_found() -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, "USB DFU interface not found")
}

fn open_usb_dev(
    vendor_id: u16,
    product_id: u16,
    bus_num: u8,
    port_num: u8,
) -> Result<rusb::DeviceHandle<rusb::Context>, dfu_libusb::Error> {
    // Due to some sort of internal caching, creating a new context each time is the most reliable.
    let ctx = rusb::Context::new().unwrap();

    // First try open_device_with_vid_pid. This should be sufficient if only one usb device with
    // the vendor_id and product_id exists
    {
        let dev = ctx
            .open_device_with_vid_pid(vendor_id, product_id)
            .ok_or(usb_dev_not_found())?;

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
            return dev.open().map_err(Into::into);
        }
    }

    Err(usb_dev_not_found().into())
}

fn open_dfu_dev(
    vendor_id: u16,
    product_id: u16,
    bus_num: u8,
    port_num: u8,
    name: &str,
) -> Result<dfu_libusb::Dfu<rusb::Context>, dfu_libusb::Error> {
    let dev = open_usb_dev(vendor_id, product_id, bus_num, port_num)?;
    let langs = dev.read_languages(DELAY)?;
    let active_desc = dev.device().active_config_descriptor()?;

    for intf in active_desc.interfaces() {
        let descs = intf.descriptors();

        for desc in descs {
            let Ok(intf_str) = dev.read_interface_string(langs[0], &desc, DELAY) else {
                continue;
            };

            if intf_str == name {
                return dfu_libusb::DfuLibusb::from_usb_device(
                    dev.device(),
                    dev,
                    desc.interface_number(),
                    desc.setting_number(),
                );
            }
        }
    }

    Err(dfu_intf_not_found().into())
}

fn open_dfu_dev_with_retry(
    vendor_id: u16,
    product_id: u16,
    bus_num: u8,
    port_num: u8,
    name: &str,
) -> Result<dfu_libusb::Dfu<rusb::Context>, dfu_libusb::Error> {
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
) -> Result<(), dfu_libusb::Error> {
    let mut dev = open_dfu_dev_with_retry(vendor_id, product_id, bus_num, port_num, &name)?;
    dev.download(reader, size)?;
    let _ = dev.usb_reset();
    let _ = dev.detach();

    Ok(())
}
