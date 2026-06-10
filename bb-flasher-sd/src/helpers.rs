use std::io::{self, Write};

use bb_helper::cancel::CancellationToken;
use tokio::sync::mpsc;
use tokio_util::io::SyncIoBridge;

pub(crate) fn chan_send(chan: Option<&mut mpsc::Sender<f32>>, msg: f32) {
    if let Some(c) = chan {
        let _ = c.try_send(msg);
    }
}

pub(crate) const fn progress(pos: u64, img_size: u64) -> f32 {
    pos as f32 / img_size as f32
}

pub(crate) fn check_cancel(tkn: Option<&CancellationToken>) -> crate::Result<()> {
    if let Some(t) = tkn
        && t.is_cancelled()
    {
        Err(crate::Error::Aborted)
    } else {
        Ok(())
    }
}

pub(crate) trait Eject {
    fn eject(self) -> io::Result<()>;
}

impl Eject for std::fs::File {
    fn eject(mut self) -> io::Result<()> {
        self.flush()?;
        self.sync_all()
    }
}

const BLOCK_SIZE: usize = 4096;

#[derive(Debug)]
/// Wrapper to perform aligned read/write operations.
pub(crate) struct DeviceWrapper<F> {
    f: F,
    offset: u64,
    buf: Box<DirectIoBuffer<BLOCK_SIZE>>,
    cache_offset: u64,
}

impl<F> DeviceWrapper<F> {
    /// Start offset of current block
    const fn block_offset(&self) -> u64 {
        self.offset - self.cache_buf_offset() as u64
    }

    /// Offset inside cache to start reading/writing
    const fn cache_buf_offset(&self) -> usize {
        (self.offset % BLOCK_SIZE as u64) as usize
    }

    /// Number of bytes from `Self::cache_buf_offset` that can be used
    const fn cache_buf_hit_len(&self) -> usize {
        self.buf.len() - self.cache_buf_offset()
    }

    pub(crate) fn into_inner(self) -> F {
        self.f
    }
}

impl<F> DeviceWrapper<F>
where
    F: io::Seek,
{
    pub(crate) fn new(mut f: F) -> io::Result<Self> {
        f.rewind()?;
        Ok(Self {
            f,
            offset: 0,
            // Hack to make reading from 0 working
            cache_offset: 1,
            buf: Box::new(DirectIoBuffer::new()),
        })
    }
}

impl<F> DeviceWrapper<F>
where
    F: io::Read + io::Seek,
{
    fn fill_cache(&mut self) -> io::Result<()> {
        if self.cache_offset != self.block_offset() {
            self.cache_offset = self.block_offset();
            self.f.seek(io::SeekFrom::Start(self.cache_offset))?;
            self.f.read_exact(self.buf.as_mut_slice())
        } else {
            Ok(())
        }
    }
}

impl<F> io::Read for DeviceWrapper<F>
where
    F: io::Read + io::Seek,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.fill_cache()?;
        let count = std::cmp::min(buf.len(), self.cache_buf_hit_len());

        buf[..count].copy_from_slice(
            &self.buf.as_slice()[self.cache_buf_offset()..(self.cache_buf_offset() + count)],
        );

        self.offset += count as u64;

        Ok(count)
    }
}

impl<F> io::Write for DeviceWrapper<F>
where
    F: io::Write + io::Read + io::Seek,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.fill_cache()?;
        let count = std::cmp::min(buf.len(), self.cache_buf_hit_len());
        let start = self.cache_buf_offset();

        self.buf.as_mut_slice()[start..(start + count)].copy_from_slice(&buf[..count]);

        self.f.seek(io::SeekFrom::Start(self.cache_offset))?;
        self.f.write(self.buf.as_slice())?;

        self.offset += count as u64;

        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.f.flush()
    }
}

impl<F> io::Seek for DeviceWrapper<F>
where
    F: io::Seek,
{
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        match pos {
            io::SeekFrom::Start(i) => self.offset = i,
            io::SeekFrom::Current(i) => self.offset = self.offset.checked_add_signed(i).unwrap(),
            io::SeekFrom::End(_) => self.offset = self.f.seek(pos)?,
        }

        Ok(self.offset)
    }
}

#[repr(align(4096))]
#[derive(Debug)]
pub(crate) struct DirectIoBuffer<const N: usize>([u8; N]);

impl<const N: usize> DirectIoBuffer<N> {
    pub(crate) const fn new() -> Self {
        Self([0u8; N])
    }

    pub(crate) const fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub(crate) const fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.0
    }

    const fn len(&self) -> usize {
        self.0.len()
    }
}

/// A wrapper to support writing the first block at the end. This is required on Windows to make
/// things work reliably.
#[derive(Debug)]
pub(crate) struct SdCardWrapper<W> {
    inner: W,
    buf: Box<DirectIoBuffer<BLOCK_SIZE>>,
    pos: u64,
}

impl<W> SdCardWrapper<W> {
    pub(crate) fn new(inner: W) -> Self {
        Self {
            inner,
            buf: Box::new(DirectIoBuffer::new()),
            pos: 0,
        }
    }
}

impl<W> SdCardWrapper<W>
where
    W: io::Write + io::Seek,
{
    fn finish(&mut self) -> io::Result<()> {
        self.inner.seek(io::SeekFrom::Start(0))?;
        self.inner.write_all(self.buf.as_slice())?;
        self.pos = u64::try_from(self.buf.len()).unwrap();

        Ok(())
    }
}

impl<W> io::Read for SdCardWrapper<W>
where
    W: io::Read + io::Seek,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let pos = usize::try_from(self.pos).unwrap();

        let count = if pos < self.buf.len() {
            let count = std::cmp::min(self.buf.len() - pos, buf.len());
            self.inner
                .seek(io::SeekFrom::Current(i64::try_from(count).unwrap()))?;
            buf[..count].copy_from_slice(&self.buf.as_slice()[pos..(pos + count)]);
            count
        } else {
            self.inner.read(buf)?
        };

        self.pos += u64::try_from(count).unwrap();
        Ok(count)
    }
}

impl<W> io::Write for SdCardWrapper<W>
where
    W: io::Write + io::Seek,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let pos = usize::try_from(self.pos).unwrap();

        let count = if pos < self.buf.len() {
            let count = std::cmp::min(self.buf.len() - pos, buf.len());
            self.inner
                .seek(io::SeekFrom::Current(i64::try_from(count).unwrap()))?;
            self.buf.as_mut_slice()[pos..(pos + count)].copy_from_slice(&buf[..count]);
            count
        } else {
            self.inner.write(buf)?
        };

        self.pos += u64::try_from(count).unwrap();
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<W> io::Seek for SdCardWrapper<W>
where
    W: io::Seek,
{
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.pos = self.inner.seek(pos)?;
        Ok(self.pos)
    }
}

impl<W> Eject for SdCardWrapper<W>
where
    W: io::Write + io::Seek + Eject,
{
    fn eject(mut self) -> io::Result<()> {
        self.finish()?;
        self.inner.eject()
    }
}

impl<W> Eject for SyncIoBridge<W>
where
    W: Unpin + Eject,
{
    fn eject(self) -> io::Result<()> {
        self.into_inner().eject()
    }
}

pub(crate) trait IntoStdIo {
    type Output: io::Read + io::Write + io::Seek + std::fmt::Debug + Send + Eject;

    fn into_std_io(self) -> impl Future<Output = io::Result<Self::Output>>;
}

impl IntoStdIo for tokio::fs::File {
    type Output = std::fs::File;

    async fn into_std_io(self) -> io::Result<Self::Output> {
        Ok(self.into_std().await)
    }
}

impl<T> IntoStdIo for tokio_util::compat::Compat<futures::io::AllowStdIo<SdCardWrapper<T>>>
where
    T: io::Read + io::Write + io::Seek + std::fmt::Debug + Send + Eject,
{
    type Output = SdCardWrapper<T>;

    async fn into_std_io(self) -> io::Result<Self::Output> {
        Ok(self.into_inner().into_inner())
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Seek, SeekFrom, Write};

    use crate::helpers::BLOCK_SIZE;

    use super::SdCardWrapper;

    const FILE_LEN: usize = 12 * 1024;

    fn test_data() -> std::io::Cursor<Box<[u8]>> {
        let data: Vec<u8> = (0..FILE_LEN)
            .map(|x| x % 255)
            .map(|x| u8::try_from(x).unwrap())
            .collect();
        std::io::Cursor::new(data.into())
    }

    fn test_file() -> super::DeviceWrapper<std::fs::File> {
        let mut data = test_data();
        let mut f = tempfile::tempfile().unwrap();

        std::io::copy(&mut data, &mut f).unwrap();
        f.rewind().unwrap();

        super::DeviceWrapper::new(f).unwrap()
    }

    #[test]
    fn dev_wrapper_read() {
        let mut temp = test_file();
        let mut buf = [0u8; 50];

        temp.seek(SeekFrom::Start(10)).unwrap();
        temp.read_exact(&mut buf).unwrap();

        let ans: Vec<u8> = (10..60).collect();
        assert_eq!(buf.as_slice(), &ans);

        temp.seek(SeekFrom::Start(4095)).unwrap();
        temp.read_exact(&mut buf).unwrap();

        let ans: Vec<u8> = (4095..4145).map(|x| (x % 255) as u8).collect();
        assert_eq!(buf.as_slice(), &ans);
    }

    #[test]
    fn dev_wrapper_write() {
        let mut temp = test_file();
        let ans = [9u8; 50];

        let mut buf = [9u8; 50];
        temp.seek(SeekFrom::Start(10)).unwrap();
        temp.write_all(&buf).unwrap();

        temp.seek(SeekFrom::Start(4090)).unwrap();
        temp.write_all(&buf).unwrap();

        temp.seek(SeekFrom::Start(10)).unwrap();
        temp.read_exact(&mut buf).unwrap();

        assert_eq!(ans, buf);

        temp.seek(SeekFrom::Start(4090)).unwrap();
        temp.read_exact(&mut buf).unwrap();

        assert_eq!(ans, buf);
    }

    #[test]
    fn sd_card_wrapper() {
        let mut test_data = test_data();
        let mut temp_buf = Vec::with_capacity(FILE_LEN);

        let f = tempfile::tempfile().unwrap();
        f.set_len(FILE_LEN as u64).unwrap();
        let mut sd = SdCardWrapper::new(f);

        std::io::copy(&mut test_data, &mut sd).unwrap();
        sd.flush().unwrap();

        // Read underlying file contents directly
        sd.inner.rewind().unwrap();
        sd.inner.read_to_end(&mut temp_buf).unwrap();

        // Everything after the cached block should already be flushed
        assert_eq!(&test_data.get_ref()[BLOCK_SIZE..], &temp_buf[BLOCK_SIZE..]);
        // First block should still only exist in cache
        assert_eq!(
            &test_data.get_ref()[..BLOCK_SIZE],
            &sd.buf.as_slice()[..BLOCK_SIZE]
        );
        assert!(temp_buf[..BLOCK_SIZE].iter().all(|x| *x == 0));

        temp_buf.clear();

        // Logical reads should still see full data
        sd.rewind().unwrap();
        sd.read_to_end(&mut temp_buf).unwrap();
        assert_eq!(temp_buf.as_slice(), test_data.get_ref().as_ref());

        // finish() flushes cached block
        sd.finish().unwrap();

        temp_buf.clear();

        sd.inner.rewind().unwrap();
        sd.inner.read_to_end(&mut temp_buf).unwrap();
        assert_eq!(temp_buf.as_slice(), test_data.get_ref().as_ref());
    }
}
