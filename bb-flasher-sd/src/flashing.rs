use std::io::{Read, Seek, Write};
use std::path::Path;
use std::sync::Weak;
use std::time::Instant;

use futures::channel::mpsc;

use crate::Result;
use crate::customization::Customization;
use crate::helpers::{DirectIoBuffer, Eject, chan_send, check_arc, progress};

// Stack overflow occurs during debug since box moves data from stack to heap in debug builds
#[cfg(not(debug_assertions))]
const BUFFER_SIZE: usize = 1 * 1024 * 1024;
#[cfg(debug_assertions)]
const BUFFER_SIZE: usize = 8 * 1024;

fn reader_task(
    mut img: impl Read,
    buf_rx: std::sync::mpsc::Receiver<Box<DirectIoBuffer<BUFFER_SIZE>>>,
    buf_tx: std::sync::mpsc::SyncSender<(Box<DirectIoBuffer<BUFFER_SIZE>>, usize)>,
    cancel: Option<Weak<()>>,
) -> Result<()> {
    while let Ok(mut buf) = buf_rx.recv() {
        let count = read_aligned(&mut img, buf.as_mut_slice())?;
        if count == 0 {
            break;
        }

        buf_tx.send((buf, count)).unwrap();
        check_arc(cancel.as_ref())?;
    }

    Ok(())
}

fn writer_task(
    img_size: u64,
    mut sd: impl Write + Seek,
    mut chan: Option<&mut mpsc::Sender<f32>>,
    buf_rx: std::sync::mpsc::Receiver<(Box<DirectIoBuffer<BUFFER_SIZE>>, usize)>,
    buf_tx: std::sync::mpsc::SyncSender<Box<DirectIoBuffer<BUFFER_SIZE>>>,
    cancel: Option<Weak<()>>,
) -> Result<()> {
    let mut pos = 0;

    while let Ok((buf, count)) = buf_rx.recv() {
        sd.write_all(&buf.as_slice()[..count])?;

        pos += count;
        // Clippy warning is simply wrong here
        #[allow(clippy::option_map_or_none)]
        chan_send(
            chan.as_mut().map_or(None, |p| Some(p)),
            progress(pos, img_size),
        );

        let _ = buf_tx.send(buf);
        check_arc(cancel.as_ref())?;
    }

    sd.flush().map_err(Into::into)
}

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
    img: impl Read + Send + 'static,
    img_size: u64,
    sd: impl Write + Seek,
    chan: Option<&mut mpsc::Sender<f32>>,
    cancel: Option<Weak<()>>,
) -> Result<()> {
    const NUM_BUFFERS: usize = 4;

    let (tx1, rx1) = std::sync::mpsc::sync_channel(NUM_BUFFERS);
    let (tx2, rx2) = std::sync::mpsc::sync_channel(NUM_BUFFERS);
    let global_start = Instant::now();

    // Starting buffers
    for _ in 0..NUM_BUFFERS {
        tx1.send(Box::new(DirectIoBuffer::new())).unwrap();
    }

    let cancle_clone = cancel.clone();
    let handle = std::thread::spawn(move || reader_task(img, rx1, tx2, cancle_clone));

    writer_task(img_size, sd, chan, rx2, tx1, cancel)?;

    tracing::info!("Total Time taken: {:?}", global_start.elapsed());

    handle.join().unwrap()
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
pub fn flash<R: Read + Send + 'static>(
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
    img: impl Read + Send + 'static,
    img_size: u64,
    sd: impl Read + Write + Seek + Eject + std::fmt::Debug,
    mut chan: Option<mpsc::Sender<f32>>,
    customization: Option<Customization>,
    cancel: Option<Weak<()>>,
) -> Result<()> {
    chan_send(chan.as_mut(), 0.0);

    let mut sd = crate::helpers::SdCardWrapper::new(sd);

    write_sd(img, img_size, &mut sd, chan.as_mut(), cancel.clone())?;

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
            .map(|x| x % 255)
            .map(|x| u8::try_from(x).unwrap())
            .collect();
        std::io::Cursor::new(data.into())
    }

    #[test]
    fn sd_write() {
        const FILE_LEN: usize = 12 * 1024;

        let dummy_file = test_file(FILE_LEN);
        let mut sd = std::io::Cursor::new(Vec::<u8>::new());

        write_sd(dummy_file.clone(), FILE_LEN as u64, &mut sd, None, None).unwrap();

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
