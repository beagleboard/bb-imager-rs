use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::Weak;

use futures::channel::mpsc;

use crate::Result;
use crate::customization::Customization;
use crate::helpers::{Eject, chan_send, check_arc, progress};

fn read_aligned(mut img: impl Read, buf: &mut [u8]) -> Result<usize> {
    let mut pos = 0;

    while pos != buf.len() {
        let count = img.read(&mut buf[pos..])?;
        // Need to align to 512 byte boundary for writing to sd card
        if count == 0 {
            buf[pos..].fill(0);
            return Ok(pos);
        }

        pos += count;
    }

    Ok(pos)
}

fn write_sd(
    mut img: impl Read,
    img_size: u64,
    mut sd: impl Write,
    mut chan: Option<&mut mpsc::Sender<f32>>,
    cancel: Option<&Weak<()>>,
) -> Result<()> {
    let mut buf = [0u8; 512];
    let mut pos = 0;

    // Clippy warning is simply wrong here
    #[allow(clippy::option_map_or_none)]
    chan_send(chan.as_mut().map_or(None, |p| Some(p)), 0.0);
    loop {
        let count = read_aligned(&mut img, &mut buf)?;
        if count == 0 {
            break;
        }

        // Since buf is 512, just write the whole thing since read_aligned will always fill it
        // fully.
        sd.write_all(&buf)?;

        pos += count;
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
    let sd = crate::pal::open(dst)?;

    let (img, img_size) = img_resolver()?;
    flash_internal(img, img_size, sd, chan, customization, cancel)
}

#[cfg(windows)]
fn flash_internal(
    mut img: impl Read,
    img_size: u64,
    mut sd: impl Write + Eject,
    mut chan: Option<mpsc::Sender<f32>>,
    customization: Option<Customization>,
    cancel: Option<Weak<()>>,
) -> Result<()> {
    tracing::info!("Creating a copy of image for modification");
    let mut img = {
        let mut temp = tempfile::tempfile().unwrap();
        std::io::copy(&mut img, &mut temp).unwrap();
        temp
    };

    chan_send(chan.as_mut(), 0.0);

    tracing::info!("Applying customization");
    if let Some(c) = customization {
        img.seek(SeekFrom::Start(0))?;
        c.customize(&mut img)?;
    }

    tracing::info!("Flashing modified image to SD card");
    img.seek(SeekFrom::Start(0))?;
    write_sd(
        std::io::BufReader::new(img),
        img_size,
        BufWriter::new(&mut sd),
        chan.as_mut(),
        cancel.as_ref(),
    )?;

    let _ = sd.eject();

    Ok(())
}

#[cfg(not(windows))]
fn flash_internal(
    img: impl Read,
    img_size: u64,
    mut sd: impl Read + Write + Seek + Eject + std::fmt::Debug,
    mut chan: Option<mpsc::Sender<f32>>,
    customization: Option<Customization>,
    cancel: Option<Weak<()>>,
) -> Result<()> {
    chan_send(chan.as_mut(), 0.0);

    write_sd(
        img,
        img_size,
        BufWriter::new(&mut sd),
        chan.as_mut(),
        cancel.as_ref(),
    )?;

    check_arc(cancel.as_ref())?;

    if let Some(c) = customization {
        sd.seek(SeekFrom::Start(0))?;
        c.customize(&mut sd)?;
    }

    let _ = sd.eject();

    Ok(())
}
