use std::io::{Read, Seek, Write};
use std::path::Path;
use std::sync::Weak;

use futures::channel::mpsc;

use crate::Result;
use crate::customization::Customization;
use crate::helpers::{Eject, chan_send, check_arc, progress};

const BUFFER_SIZE: usize = 1 * 1024 * 1024;
const ALIGNMENT: usize = 512;

#[repr(align(512))]
struct DirectIoBuffer([u8; BUFFER_SIZE]);

impl DirectIoBuffer {
    const fn new() -> Self {
        Self([0u8; BUFFER_SIZE])
    }
}

fn write_sd(
    mut img: impl Read,
    img_size: u64,
    mut sd: impl Write + Seek,
    mut chan: Option<&mut mpsc::Sender<f32>>,
    cancel: Option<&Weak<()>>,
) -> Result<()> {
    let mut buf = Box::new(DirectIoBuffer::new());
    let mut pos = 0;

    // Clippy warning is simply wrong here
    #[allow(clippy::option_map_or_none)]
    chan_send(chan.as_mut().map_or(None, |p| Some(p)), 0.0);
    loop {
        let buf_pos = pos % ALIGNMENT;

        let count = img.read(&mut buf.0[buf_pos..])?;
        if count == 0 {
            // Pad remaining data with 0 and write
            if buf_pos != 0 {
                buf.0[buf_pos..ALIGNMENT].fill(0);
                sd.write_all(&mut buf.0[..ALIGNMENT])?;
            }

            break;
        }

        // Can only write in ALIGNMENT byte chunks
        let bytes_to_write = buf_pos + count - count % ALIGNMENT;
        sd.write_all(&buf.0[..bytes_to_write])?;

        // Copy the remaining bytes to buffer start and update position in buffer
        buf.0
            .copy_within(bytes_to_write..(bytes_to_write + count % ALIGNMENT), 0);

        pos += bytes_to_write;
        // Clippy warning is simply wrong here
        #[allow(clippy::option_map_or_none)]
        chan_send(
            chan.as_mut().map_or(None, |p| Some(p)),
            progress(pos, img_size),
        );
        check_arc(cancel)?;
    }

    sd.flush().map_err(Into::into)
}

/// Flash OS image to SD card.
///
/// # Customization
///
/// Support post flashing customization. Currently only sysconf is supported, which is used by
/// [BeagleBoard.org].
///
/// # Image
///
/// Using a resolver function for image and image size. This is to allow downloading the image, or
/// some kind of lazy loading after SD card permissions have be acquired. This is useful in GUIs
/// since the user would expect a password prompt at the start of flashing.
///
/// Many users might switch task after starting the flashing process, which would make it
/// frustrating if the prompt occured after downloading.
///
/// # Progress
///
/// Progress lies between 0 and 1.
///
/// # Aborting
///
/// The process can be aborted by dropping all strong references to the [`Arc`] that owns the
/// [`Weak`] passed as `cancel`.
///
/// [`Arc`]: std::sync::Arc
/// [`Weak`]: std::sync::Weak
/// [BeagleBoard.org]: https://www.beagleboard.org/
pub fn flash<R: Read>(
    img_resolver: impl FnOnce() -> std::io::Result<(R, u64)>,
    dst: &Path,
    chan: Option<mpsc::Sender<f32>>,
    customization: Option<Customization>,
    cancel: Option<Weak<()>>,
) -> Result<()> {
    if let Some(x) = &customization {
        if !x.validate() {
            return Err(crate::Error::InvalidCustomizaton);
        }
    }

    let sd = crate::pal::open(dst)?;

    let (img, img_size) = img_resolver()?;
    flash_internal(img, img_size, sd, chan, customization, cancel)
}

fn flash_internal(
    img: impl Read,
    img_size: u64,
    mut sd: impl Read + Write + Seek + Eject + std::fmt::Debug,
    mut chan: Option<mpsc::Sender<f32>>,
    customization: Option<Customization>,
    cancel: Option<Weak<()>>,
) -> Result<()> {
    chan_send(chan.as_mut(), 0.0);

    write_sd(img, img_size, &mut sd, chan.as_mut(), cancel.as_ref())?;

    check_arc(cancel.as_ref())?;

    if let Some(c) = customization {
        let temp = crate::helpers::DeviceWrapper::new(&mut sd).unwrap();
        c.customize(temp)?;
    }

    let _ = sd.eject();

    Ok(())
}
