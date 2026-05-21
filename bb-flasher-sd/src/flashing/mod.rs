use std::io::Read;
use std::time::Instant;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio_util::compat::FuturesAsyncReadCompatExt;

use crate::Result;
use crate::customization::Customization;
use crate::helpers::{DirectIoBuffer, EjectAsync, chan_send, progress};

#[cfg(test)]
mod tests;

// Stack overflow occurs during debug since box moves data from stack to heap in debug builds
#[cfg(not(debug_assertions))]
const BUFFER_SIZE: usize = 1 * 1024 * 1024;
#[cfg(debug_assertions)]
const BUFFER_SIZE: usize = 8 * 1024;

async fn reader_task_async(
    mut img: impl AsyncRead + Unpin,
    mut buf_rx: mpsc::Receiver<Box<DirectIoBuffer<BUFFER_SIZE>>>,
    buf_tx: mpsc::Sender<(Box<DirectIoBuffer<BUFFER_SIZE>>, usize)>,
) -> Result<()> {
    while let Some(mut buf) = buf_rx.recv().await {
        let count = read_aligned_async(&mut img, buf.as_mut_slice()).await?;
        if count == 0 {
            break;
        }

        buf_tx
            .send((buf, count))
            .await
            .map_err(|_| crate::Error::WriterClosed)?;
    }

    Ok(())
}

/// While writing, a few assumptions should hold:
/// - All writes should be in buffers multiple of block size (4K).
/// - All writes should be aligned to block size (4K).
///
/// Thus, we will be writing some data that is not strictly present in the bmap.
async fn writer_task_bmap_async<Sd>(
    bmap: bb_bmap_parser::Bmap,
    mut sd: Sd,
    mut chan: Option<mpsc::Sender<f32>>,
    mut buf_rx: mpsc::Receiver<(Box<DirectIoBuffer<BUFFER_SIZE>>, usize)>,
    buf_tx: mpsc::Sender<Box<DirectIoBuffer<BUFFER_SIZE>>>,
) -> Result<Sd>
where
    Sd: AsyncWrite + AsyncSeek + Unpin,
{
    let mut pos = 0;
    let (mut buf, mut count) = buf_rx.recv().await.unwrap();
    let img_size = bmap.total_mapped_size();
    let mut bytes_written = 0u64;

    for b in bmap.block_map() {
        let end_offset = b.offset() + b.length();

        loop {
            tracing::debug!(
                "pos: {}, bytes_written: {}, end_offset: {}",
                pos,
                bytes_written,
                end_offset
            );
            // Write any buffer that lies even partially in the bmap range.
            if pos + (count as u64) > b.offset() && pos < end_offset {
                sd.seek(std::io::SeekFrom::Start(pos)).await?;
                sd.write_all(&buf.as_slice()[..count]).await?;
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

            match buf_rx.recv().await {
                Some((x, y)) => {
                    let _ = buf_tx.send(buf).await;
                    buf = x;
                    count = y;
                }
                None => break,
            }
        }
    }

    sd.flush().await?;

    Ok(sd)
}

async fn writer_task_async<Sd>(
    img_size: u64,
    mut sd: Sd,
    mut chan: Option<mpsc::Sender<f32>>,
    mut buf_rx: mpsc::Receiver<(Box<DirectIoBuffer<BUFFER_SIZE>>, usize)>,
    buf_tx: mpsc::Sender<Box<DirectIoBuffer<BUFFER_SIZE>>>,
) -> Result<Sd>
where
    Sd: AsyncWrite + Unpin,
{
    let mut pos = 0u64;

    while let Some((buf, count)) = buf_rx.recv().await {
        sd.write_all(&buf.as_slice()[..count]).await?;

        pos += count as u64;
        // Clippy warning is simply wrong here
        #[allow(clippy::option_map_or_none)]
        chan_send(
            chan.as_mut().map_or(None, |p| Some(p)),
            progress(pos, img_size),
        );

        let _ = buf_tx.send(buf).await;
    }

    sd.flush().await?;

    Ok(sd)
}

/// A lot of reads from compressed files are not aligned. Since reading even from compressed files
/// is significantly faster than writing to SD Card, better to do multiple reads.
async fn read_aligned_async(mut img: impl AsyncRead + Unpin, buf: &mut [u8]) -> Result<usize> {
    const ALIGNMENT: usize = 512;

    let mut pos = 0;

    while pos != buf.len() {
        let count = img.read(&mut buf[pos..]).await?;
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

async fn write_sd_async<Sd>(
    img: impl AsyncRead + Unpin + Send + 'static,
    img_size: u64,
    bmap: Option<bb_bmap_parser::Bmap>,
    sd: Sd,
    chan: Option<mpsc::Sender<f32>>,
) -> Result<Sd>
where
    Sd: AsyncWrite + AsyncSeek + Unpin + Send + 'static,
{
    const NUM_BUFFERS: usize = 4;

    let (tx1, rx1) = mpsc::channel(NUM_BUFFERS);
    let (tx2, rx2) = mpsc::channel(NUM_BUFFERS);
    let global_start = Instant::now();

    // Starting buffers
    for _ in 0..NUM_BUFFERS {
        tx1.send(Box::new(DirectIoBuffer::new())).await.unwrap();
    }

    let mut reader = tokio::spawn(reader_task_async(img, rx1, tx2));
    let mut writer = match bmap {
        Some(x) => tokio::spawn(writer_task_bmap_async(x, sd, chan, rx2, tx1)),
        None => tokio::spawn(writer_task_async(img_size, sd, chan, rx2, tx1)),
    };

    let res = tokio::select! {
        r = &mut reader => {
            match r.unwrap() {
                Ok(()) => {
                    writer.await.unwrap()
                }
                Err(e) => {
                    writer.abort();
                    tracing::error!("Reader failed");
                    Err(e)
                }
            }
        }

        r = &mut writer => {
            match r.unwrap() {
                Ok(sd) => {
                    reader.await.unwrap()?;
                    Ok(sd)
                }
                Err(e) => {
                    reader.abort();
                    tracing::error!("Writer failed");
                    Err(e)
                }
            }
        }
    };

    tracing::info!("Total Time taken: {:?}", global_start.elapsed());

    res
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
pub async fn flash<R: Read + Send + 'static>(
    img: impl Future<Output = std::io::Result<(R, u64)>>,
    bmap: Option<impl Future<Output = std::io::Result<Box<str>>>>,
    dst: crate::Destination,
    chan: Option<mpsc::Sender<f32>>,
    customizations: Vec<Customization>,
    _cancel: Option<tokio_util::sync::CancellationToken>,
) -> Result<()> {
    flash_async(
        async move {
            img.await
                .map(|(r, size)| (futures::io::AllowStdIo::new(r).compat(), size))
        },
        bmap,
        dst,
        chan,
        customizations,
    )
    .await
}

pub async fn flash_async<R: AsyncRead + Send + Unpin + 'static>(
    img: impl Future<Output = std::io::Result<(R, u64)>>,
    bmap: Option<impl Future<Output = std::io::Result<Box<str>>>>,
    dst: crate::Destination,
    chan: Option<mpsc::Sender<f32>>,
    customizations: Vec<Customization>,
) -> Result<()> {
    tracing::info!("Opening Destination");

    match dst {
        crate::Destination::File(path) => {
            let sd = tokio::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)
                .await?;
            flash_internal_async(img, bmap, sd, chan, customizations).await
        }
        crate::Destination::SdCard(path) => {
            let sd = crate::pal::open(&path).await?;
            flash_internal_async(img, bmap, sd, chan, customizations).await
        }
    }
}

async fn flash_internal_async<R: AsyncRead + Send + Unpin + 'static>(
    img: impl Future<Output = std::io::Result<(R, u64)>>,
    bmap: Option<impl Future<Output = std::io::Result<Box<str>>>>,
    sd: impl AsyncRead + AsyncWrite + AsyncSeek + EjectAsync + std::fmt::Debug + Send + Unpin + 'static,
    mut chan: Option<mpsc::Sender<f32>>,
    customizations: Vec<Customization>,
) -> Result<()> {
    tracing::info!("Resolving Image");
    let bmap = match bmap {
        Some(x) => {
            Some(bb_bmap_parser::Bmap::from_xml(&x.await?).map_err(|_| crate::Error::InvalidBmap)?)
        }
        None => None,
    };
    let (img, img_size) = img.await?;

    chan_send(chan.as_mut(), 0.0);

    let sd = crate::helpers::SdCardWrapperAsync::new(sd);

    tracing::info!("Writing to SD Card");
    let sd = write_sd_async(img, img_size, bmap, sd, chan).await?;

    tracing::info!("Applying customization");
    let mut temp = crate::helpers::DeviceWrapperAsync::new(sd).await.unwrap();
    let sd = tokio::task::spawn_blocking(move || {
        for c in customizations {
            c.customize_async(&mut temp)?;
        }
        Ok::<_, crate::Error>(temp)
    })
    .await
    .unwrap()?;

    tracing::info!("Ejecting SD Card");
    let _ = sd.eject().await;

    Ok(())
}
