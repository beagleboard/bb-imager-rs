use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::Weak;

use futures::channel::mpsc;
use sha2::{Digest, Sha256};

use crate::customization::Customization;
use crate::helpers::{chan_send, check_arc, progress, sha256_reader_progress};
use crate::{Error, Result, Status};

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
    mut hasher: Option<&mut Sha256>,
    mut chan: Option<&mut mpsc::Sender<Status>>,
    cancel: Option<&Weak<()>>,
) -> Result<()> {
    let mut buf = [0u8; 512];
    let mut pos = 0;

    chan_send(
        chan.as_mut().map_or(None, |p| Some(p)),
        Status::Flashing(0.0),
    );
    loop {
        let count = read_aligned(&mut img, &mut buf)?;
        if count == 0 {
            break;
        }

        // Since buf is 512, just write the whole thing since read_aligned will always fill it
        // fully.
        sd.write_all(&buf)?;
        if let Some(ref mut h) = hasher {
            h.update(&buf[..count]);
        }

        pos += count;
        chan_send(
            chan.as_mut().map_or(None, |p| Some(p)),
            Status::Flashing(progress(pos, img_size)),
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
    verify: bool,
    mut chan: Option<mpsc::Sender<Status>>,
    customization: Option<Customization>,
    cancel: Option<Weak<()>>,
) -> Result<()> {
    let mut sd = crate::pal::open(dst)?;
    let (img, img_size) = img_resolver()?;

    chan_send(chan.as_mut(), Status::Flashing(0.0));
    if verify {
        let mut hasher = Sha256::new();
        write_sd(
            img,
            img_size,
            BufWriter::new(&mut sd),
            Some(&mut hasher),
            chan.as_mut(),
            cancel.as_ref(),
        )?;

        let img_hash: [u8; 32] = hasher
            .finalize()
            .as_slice()
            .try_into()
            .expect("SHA-256 is 32 bytes");

        // Start verification
        chan_send(chan.as_mut(), Status::Verifying(0.0));
        sd.seek(std::io::SeekFrom::Start(0))?;
        let hash =
            sha256_reader_progress(BufReader::new((&mut sd).take(img_size)), img_size, chan)?;

        check_arc(cancel.as_ref())?;
        if hash != img_hash {
            tracing::debug!("Sd SHA256: {:?}", hash);
            return Err(Error::Sha256Verification);
        }
    } else {
        write_sd(
            img,
            img_size,
            BufWriter::new(&mut sd),
            None,
            chan.as_mut(),
            cancel.as_ref(),
        )?;
    }

    check_arc(cancel.as_ref())?;

    if let Some(c) = customization {
        sd.seek(SeekFrom::Start(0))?;
        c.customize(sd)?;
    }

    Ok(())
}
