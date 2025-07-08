use std::io::{Read, Seek, Write};
use std::path::Path;
use std::sync::Weak;
use std::time::{Duration, Instant};

use futures::channel::mpsc;

use crate::Result;
use crate::customization::Customization;
use crate::helpers::{DirectIoBuffer, Eject, chan_send, check_arc, progress};

// Stack overflow occurs during debug since box moves data from stack to heap in debug builds
#[cfg(not(debug_assertions))]
const BUFFER_SIZE: usize = 1 * 1024 * 1024;
#[cfg(debug_assertions)]
const BUFFER_SIZE: usize = 8 * 1024;

/// A lot of reads from compressed files are not aligned. Since reading even from compressed files
/// is significantly faster than writing to SD Card, better to do multiple reads.
fn read_aligned(mut img: impl Read, buf: &mut [u8]) -> Result<usize> {
    const ALIGNMENT: usize = 512;

    let mut pos = 0;

    while pos != buf.len() {
        let count = img.read(&mut buf[pos..])?;
        if count == 0 {
            if pos % ALIGNMENT != 0 {
                let end = pos - pos % ALIGNMENT + ALIGNMENT;
                buf[pos..end].fill(0);
                pos = end;
            }
            return Ok(pos);
        }
        pos += count;
    }

    Ok(pos)
}

fn write_sd(
    mut img: impl Read,
    img_size: u64,
    mut sd: impl Write + Seek,
    mut chan: Option<&mut mpsc::Sender<f32>>,
    cancel: Option<&Weak<()>>,
) -> Result<()> {
    let mut buf = Box::new(DirectIoBuffer::<BUFFER_SIZE>::new());
    let mut pos = 0;
    let global_start = Instant::now();
    let mut reading_time = Duration::from_secs(0);
    let mut writing_time = Duration::from_secs(0);

    // Clippy warning is simply wrong here
    #[allow(clippy::option_map_or_none)]
    chan_send(chan.as_mut().map_or(None, |p| Some(p)), 0.0);
    loop {
        let read_start = Instant::now();
        let count = read_aligned(&mut img, buf.as_mut_slice())?;
        reading_time += read_start.elapsed();
        if count == 0 {
            break;
        }

        // Since buf is 512 byte aligned, just write the whole thing since read_aligned will always
        // fill it fully.
        let write_start = Instant::now();
        sd.write_all(&buf.as_slice()[..count])?;
        writing_time += write_start.elapsed();

        pos += count;
        // Clippy warning is simply wrong here
        #[allow(clippy::option_map_or_none)]
        chan_send(
            chan.as_mut().map_or(None, |p| Some(p)),
            progress(pos, img_size),
        );
        check_arc(cancel)?;
    }

    tracing::info!("Total Time taken: {:?}", global_start.elapsed());
    tracing::info!("Reading Time: {:?}", reading_time);
    tracing::info!("Writing Time: {:?}", writing_time);

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

#[cfg(test)]
mod tests {
    use crate::flashing::read_aligned;

    use super::write_sd;

    fn test_file(len: usize) -> std::io::Cursor<Box<[u8]>> {
        let data: Vec<u8> = (0..len)
            .into_iter()
            .map(|x| x % 255)
            .map(|x| u8::try_from(x).unwrap())
            .collect();
        std::io::Cursor::new(data.into())
    }

    #[test]
    fn sd_write() {
        const FILE_LEN: usize = 12 * 1024;

        let mut dummy_file = test_file(FILE_LEN);
        let mut sd = std::io::Cursor::new(Vec::<u8>::new());

        write_sd(&mut dummy_file, FILE_LEN as u64, &mut sd, None, None).unwrap();

        assert_eq!(sd.get_ref().as_slice(), dummy_file.get_ref().as_ref());
    }

    struct UnalignedReader(std::io::Cursor<Box<[u8]>>);

    impl UnalignedReader {
        const fn as_slice(&self) -> &[u8] {
            self.0.get_ref()
        }
    }

    impl std::io::Read for UnalignedReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let count = std::cmp::min(self.0.get_ref().len() - self.0.position() as usize, 3);
            let count = std::cmp::min(count, buf.len());
            self.0.read(&mut buf[..count])
        }
    }

    #[test]
    fn aligned_read() {
        const FILE_LEN: usize = 12 * 1024;

        let mut dummy_file = UnalignedReader(test_file(FILE_LEN));
        let mut buf = [0u8; 1024];
        let mut pos = 0;

        loop {
            let count = read_aligned(&mut dummy_file, &mut buf).unwrap();
            if count == 0 {
                break;
            }

            assert_eq!(count % 512, 0);
            assert_eq!(buf[..count], dummy_file.as_slice()[pos..(pos + count)]);
            pos += count;
        }

        assert_eq!(pos, FILE_LEN);
    }
}
