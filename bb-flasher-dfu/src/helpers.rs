use std::io;

pub(crate) fn not_found() -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, "USB device not found")
}

pub(crate) fn dfu_to_io_error(value: dfu_libusb::Error) -> io::Error {
    match value {
        dfu_libusb::Error::CouldNotOpenDevice => io::Error::other("Could not open device"),
        dfu_libusb::Error::Dfu(error) => io::Error::other(format!("Dfu error: {}", error)),
        dfu_libusb::Error::Io(error) => error,
        dfu_libusb::Error::LibUsb(error) => match error {
            rusb::Error::InvalidParam => {
                io::Error::new(io::ErrorKind::InvalidInput, "Invalid parameter")
            }
            rusb::Error::Access => io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Access denied (insufficient permissions)",
            ),
            rusb::Error::NoDevice => io::Error::new(
                io::ErrorKind::ConnectionReset,
                "No such device (it may have been disconnected)",
            ),
            rusb::Error::NotFound => not_found(),
            rusb::Error::Busy => io::Error::new(io::ErrorKind::ResourceBusy, "Resource busy"),
            rusb::Error::Timeout => io::Error::new(io::ErrorKind::TimedOut, "Operation timed out"),
            rusb::Error::Overflow => io::Error::new(io::ErrorKind::FileTooLarge, "Overflow"),
            rusb::Error::Pipe => io::Error::new(io::ErrorKind::BrokenPipe, "Pipe error"),
            rusb::Error::Interrupted => {
                io::Error::new(io::ErrorKind::Interrupted, "rusb interrupted")
            }
            rusb::Error::NoMem => io::Error::new(io::ErrorKind::OutOfMemory, "No memory"),
            rusb::Error::NotSupported => {
                io::Error::new(io::ErrorKind::Unsupported, "Unsupported by rusb")
            }
            rusb::Error::BadDescriptor => {
                io::Error::new(io::ErrorKind::InvalidData, "Invalid file descriptor")
            }
            rusb::Error::Other => io::Error::other("Unknown usb error"),
            rusb::Error::Io => io::Error::other("Rusb IO error"),
        },
        dfu_libusb::Error::MissingLanguage => {
            io::Error::new(io::ErrorKind::InvalidInput, "Missing usb language")
        }
        dfu_libusb::Error::InvalidInterface => {
            io::Error::new(io::ErrorKind::InvalidInput, "Invalid usb interface")
        }
        dfu_libusb::Error::InvalidAlt => {
            io::Error::new(io::ErrorKind::InvalidInput, "Invalid usb alt")
        }
        dfu_libusb::Error::FunctionalDescriptor(error) => io::Error::other(error),
        dfu_libusb::Error::NoDfuCapableDeviceFound => not_found(),
    }
}

pub(crate) fn check_token(cancel: Option<&tokio_util::sync::CancellationToken>) -> io::Result<()> {
    match cancel {
        Some(x) if x.is_cancelled() => Err(io::Error::new(io::ErrorKind::Interrupted, "Aborted")),
        _ => Ok(()),
    }
}

pub(crate) fn is_dfu_device<U: rusb::UsbContext>(x: &rusb::Device<U>) -> bool {
    if let Ok(cfg_desc) = x.active_config_descriptor() {
        for intf in cfg_desc.interfaces() {
            for desc in intf.descriptors() {
                if desc.class_code() == 0xfe && desc.sub_class_code() == 1 {
                    return true;
                }
            }
        }
    }

    false
}

pub(crate) struct ReaderWithProgress<R: std::io::Read> {
    reader: R,
    pos: u64,
    size: u64,
    chan: tokio::sync::mpsc::Sender<f32>,
}

impl<R: std::io::Read> ReaderWithProgress<R> {
    pub(crate) const fn new(reader: R, size: u64, chan: tokio::sync::mpsc::Sender<f32>) -> Self {
        Self {
            reader,
            size,
            chan,
            pos: 0,
        }
    }
}

impl<R: std::io::Read> std::io::Read for ReaderWithProgress<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let count = self.reader.read(buf)?;

        self.pos += count as u64;
        let _ = self.chan.try_send(self.pos as f32 / self.size as f32);

        Ok(count)
    }
}
