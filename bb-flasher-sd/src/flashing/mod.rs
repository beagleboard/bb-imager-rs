use std::io::{Read, Seek, Write};
use std::sync::mpsc;
use std::time::Instant;

use bb_helper::cancel::CancellationToken;

use crate::Result;
use crate::customization::Customization;
use crate::helpers::{DirectIoBuffer, Eject, chan_send, check_cancel, progress};

#[cfg(test)]
mod tests;

// Stack overflow occurs during debug since box moves data from stack to heap in debug builds
#[cfg(not(debug_assertions))]
const BUFFER_SIZE: usize = 1 * 1024 * 1024;
#[cfg(debug_assertions)]
const BUFFER_SIZE: usize = 8 * 1024;

fn reader_task(
    mut img: impl Read,
    buf_rx: mpsc::Receiver<Box<DirectIoBuffer<BUFFER_SIZE>>>,
    buf_tx: mpsc::SyncSender<(Box<DirectIoBuffer<BUFFER_SIZE>>, usize)>,
    cancel: Option<CancellationToken>,
) -> Result<()> {
    while let Ok(mut buf) = buf_rx.recv() {
        let count = read_aligned(&mut img, buf.as_mut_slice())?;
        if count == 0 {
            break;
        }

        buf_tx
            .send((buf, count))
            .map_err(|_| crate::Error::WriterClosed)?;
        check_cancel(cancel.as_ref())?;
    }

    Ok(())
}

/// While writing, a few assumptions should hold:
/// - All writes should be in buffers multiple of block size (4K).
/// - All writes should be aligned to block size (4K).
///
/// Thus, we will be writing some data that is not strictly present in the bmap.
fn writer_task_bmap(
    bmap: bb_bmap_parser::Bmap,
    mut sd: impl Write + Seek,
    mut chan: Option<mpsc::SyncSender<f32>>,
    buf_rx: mpsc::Receiver<(Box<DirectIoBuffer<BUFFER_SIZE>>, usize)>,
    buf_tx: mpsc::SyncSender<Box<DirectIoBuffer<BUFFER_SIZE>>>,
    cancel: Option<CancellationToken>,
) -> Result<()> {
    let mut pos = 0;
    let (mut buf, mut count) = buf_rx.recv().unwrap();
    let img_size = bmap.total_mapped_size();
    let mut bytes_written = 0u64;

    for b in bmap.block_map() {
        let end_offset = b.offset() + b.length();

        loop {
            // Write any buffer that lies even partially in the bmap range.
            if pos + (count as u64) > b.offset() && pos < end_offset {
                sd.seek(std::io::SeekFrom::Start(pos))?;
                sd.write_all(&buf.as_slice()[..count])?;
                bytes_written += count as u64;
            } else if pos >= end_offset {
                break;
            }

            pos += count as u64;
            // Clippy warning is simply wrong here
            #[allow(clippy::option_map_or_none)]
            chan_send(
                chan.as_mut().map_or(None, |p| Some(p)),
                progress(bytes_written, img_size),
            );
            check_cancel(cancel.as_ref())?;

            match buf_rx.recv() {
                Ok((x, y)) => {
                    let _ = buf_tx.send(buf);
                    buf = x;
                    count = y;
                }
                Err(_) => break,
            }
        }
    }

    sd.flush().map_err(Into::into)
}

fn writer_task(
    img_size: u64,
    mut sd: impl Write + Seek,
    mut chan: Option<mpsc::SyncSender<f32>>,
    buf_rx: mpsc::Receiver<(Box<DirectIoBuffer<BUFFER_SIZE>>, usize)>,
    buf_tx: mpsc::SyncSender<Box<DirectIoBuffer<BUFFER_SIZE>>>,
    cancel: Option<CancellationToken>,
) -> Result<()> {
    let mut pos = 0u64;

    while let Ok((buf, count)) = buf_rx.recv() {
        sd.write_all(&buf.as_slice()[..count])?;

        pos += count as u64;
        // Clippy warning is simply wrong here
        #[allow(clippy::option_map_or_none)]
        chan_send(chan.as_mut(), progress(pos, img_size));

        let _ = buf_tx.send(buf);
        check_cancel(cancel.as_ref())?;
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
    img: impl Read + Send,
    img_size: u64,
    bmap: Option<bb_bmap_parser::Bmap>,
    sd: impl Write + Seek,
    chan: Option<mpsc::SyncSender<f32>>,
    cancel: Option<CancellationToken>,
) -> Result<()> {
    const NUM_BUFFERS: usize = 4;

    let (tx1, rx1) = std::sync::mpsc::sync_channel(NUM_BUFFERS);
    let (tx2, rx2) = std::sync::mpsc::sync_channel(NUM_BUFFERS);
    let global_start = Instant::now();

    // Starting buffers
    for _ in 0..NUM_BUFFERS {
        tx1.send(Box::new(DirectIoBuffer::new())).unwrap();
    }

    std::thread::scope(|s| {
        let cancle_clone = cancel.clone();
        let handle = s.spawn(move || reader_task(img, rx1, tx2, cancle_clone));

        match bmap {
            Some(x) => writer_task_bmap(x, sd, chan, rx2, tx1, cancel),
            None => writer_task(img_size, sd, chan, rx2, tx1, cancel),
        }?;
        tracing::info!("Total Time taken: {:?}", global_start.elapsed());

        handle.join().unwrap()
    })
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
pub async fn flash<R: Read + Send + 'static, B, C>(
    img: impl Future<Output = std::io::Result<(R, u64)>>,
    bmap: Option<B>,
    dst: crate::Destination,
    chan: Option<mpsc::SyncSender<f32>>,
    customizations: impl Iterator<Item = Customization<C>> + Send + 'static,
) -> Result<()>
where
    C: Iterator<Item = (Box<str>, crate::ContentType<'static>)> + Send + 'static,
    B: FnOnce() -> std::io::Result<Box<str>> + Send + 'static,
{
    tracing::info!("Opening Destination");

    match dst {
        crate::Destination::File(path) => {
            let sd = tokio::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)
                .await?
                .into_std()
                .await;
            flash_internal(img, bmap, sd, chan, customizations).await
        }
        crate::Destination::SdCard(path) => {
            let sd = crate::pal::open(&path).await?;
            let sd = crate::helpers::SdCardWrapper::new(sd);
            flash_internal(img, bmap, sd, chan, customizations).await
        }
    }
}

async fn flash_internal<R, B, Sd, C>(
    img: impl Future<Output = std::io::Result<(R, u64)>>,
    bmap: Option<B>,
    mut sd: Sd,
    mut chan: Option<mpsc::SyncSender<f32>>,
    customizations: impl Iterator<Item = Customization<C>> + Send + 'static,
) -> Result<()>
where
    R: Read + Send + 'static,
    Sd: Read + Write + Seek + Eject + std::fmt::Debug + Send + 'static,
    C: Iterator<Item = (Box<str>, crate::ContentType<'static>)> + Send + 'static,
    B: FnOnce() -> std::io::Result<Box<str>> + Send + 'static,
{
    tracing::info!("Resolving Image");
    let (img, img_size) = img.await?;

    tokio::task::spawn_blocking(move || {
        let bmap = match bmap {
            Some(x) => {
                Some(bb_bmap_parser::Bmap::from_xml(&x()?).map_err(|_| crate::Error::InvalidBmap)?)
            }
            None => None,
        };

        chan_send(chan.as_mut(), 0.0);

        tracing::info!("Writing to SD Card");
        write_sd(img, img_size, bmap, &mut sd, chan, None)?;

        tracing::info!("Applying customization");
        let mut sd = crate::helpers::DeviceWrapper::new(sd).unwrap();
        for c in customizations {
            c.customize(&mut sd, None)?;
        }

        tracing::info!("Ejecting SD Card");
        let _ = sd.into_inner().eject();

        Ok(())
    })
    .await
    .unwrap()
}
