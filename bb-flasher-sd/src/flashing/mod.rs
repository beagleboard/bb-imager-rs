use std::io::{Read, Seek, Write};
use std::sync::mpsc;
use std::time::Instant;

use bb_helper::cancel::CancellationToken;

use crate::customization::Customization;
use crate::helpers::{DirectIoBuffer, Eject, chan_send, check_cancel, progress};
use crate::{ContentType, Result};

#[cfg(test)]
mod tests;

// Stack overflow occurs during debug since box moves data from stack to heap in debug builds
#[cfg(not(debug_assertions))]
const BUFFER_SIZE: usize = 1024 * 1024;
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
    tracing::info!("Writing with bmap file.");

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
            chan_send(chan.as_mut(), progress(bytes_written, img_size));
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
    tracing::info!("Writing without bmap file. Will be slow.");

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

/// Uninhabited stand-in for [`flash`]'s `bootfs` archive type.
///
/// The `bootfs` parameter is generic, so a bare `None` leaves the compiler with
/// nothing to infer the archive type from. Pass [`NO_BOOTFS`] instead of naming
/// this type directly.
#[derive(Debug)]
pub enum NoBootfs {}

impl<'b> IntoIterator for &'b mut NoBootfs {
    type Item = (Box<str>, ContentType<'b>);
    type IntoIter = std::iter::Empty<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        match *self {}
    }
}

/// Value to pass as [`flash`]'s `bootfs` argument when the BOOT partition needs
/// no extra files written after the image.
pub const NO_BOOTFS: Option<fn() -> std::io::Result<NoBootfs>> = None;

pub struct Flasher<I, B, M, C> {
    img: I,
    bootfs: Option<B>,
    bmap: Option<M>,
    customizations: C,
    chan: Option<mpsc::SyncSender<f32>>,
    cancel: Option<CancellationToken>,
}

impl<'a, I, B, M, C, R, Be, Cu> Flasher<I, B, M, C>
where
    I: FnOnce() -> std::io::Result<(R, u64)> + Send,
    B: FnOnce() -> std::io::Result<Be>,
    M: FnOnce() -> std::io::Result<Box<str>> + Send,
    C: Iterator<Item = Customization<Cu>> + Send,
    R: Read + Send,
    for<'b> &'b mut Be: IntoIterator<Item = (Box<str>, ContentType<'b>)>,
    Cu: Iterator<Item = (Box<str>, crate::ContentType<'a>)> + Send,
{
    pub fn new(
        img: I,
        bootfs: Option<B>,
        bmap: Option<M>,
        customizations: C,
        chan: Option<mpsc::SyncSender<f32>>,
        cancel: Option<CancellationToken>,
    ) -> Self {
        Self {
            img,
            bootfs,
            bmap,
            customizations,
            chan,
            cancel,
        }
    }

    pub fn flash(self, dst: crate::Destination, resize: bool) -> Result<()> {
        tracing::info!("Opening Destination");

        match dst {
            crate::Destination::File(path) => {
                let sd = std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(path)?;
                self.flash_internal(sd, None)
            }
            crate::Destination::SdCard(path) => {
                let sd = crate::pal::open(&path)?;
                let sd = crate::helpers::SdCardWrapper::new(sd);
                let resize_size = if resize {
                    let temp = bb_drivelist::drive_list()
                        .unwrap()
                        .into_iter()
                        .find(|x| *x.raw == *path)
                        .and_then(|x| x.size)
                        .ok_or(crate::Error::InvalidDestionation)?;

                    Some(temp)
                } else {
                    None
                };

                self.flash_internal(sd, resize_size)
            }
        }
    }

    fn flash_internal(
        mut self,
        mut sd: impl Read + Write + Seek + Eject,
        sd_size_for_resize: Option<u64>,
    ) -> Result<()> {
        tracing::info!("Resolving Bmap");
        let bmap = match self.bmap {
            Some(x) => {
                Some(bb_bmap_parser::Bmap::from_xml(&x()?).map_err(|_| crate::Error::InvalidBmap)?)
            }
            None => None,
        };
        tracing::info!("Resolving Image");
        let (img, img_size) = (self.img)()?;

        chan_send(self.chan.as_mut(), 0.0);

        tracing::info!("Writing to SD Card");
        write_sd(img, img_size, bmap, &mut sd, self.chan, self.cancel.clone())?;

        if let Some(boot_cb) = self.bootfs {
            tracing::info!("Applying bootfs updates");
            let mut bootfs_img = boot_cb()?;
            crate::bootfs_update::internal(
                (&mut bootfs_img).into_iter(),
                &mut sd,
                self.cancel.clone(),
            )?;
        }

        tracing::info!("Applying customization");
        let mut sd = crate::helpers::DeviceWrapper::new(sd).unwrap();
        for c in self.customizations {
            check_cancel(self.cancel.as_ref())?;
            c.customize(&mut sd, None)?;
        }

        if let Some(x) = sd_size_for_resize {
            tracing::info!("Resize last partition");
            crate::customization::resize_last_partition(&mut sd, x)?;
        }

        tracing::info!("Ejecting SD Card");
        let _ = sd.into_inner().eject();

        Ok(())
    }
}
