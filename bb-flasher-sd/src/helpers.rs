use std::io;

use futures::channel::mpsc;

use crate::Result;

pub(crate) fn chan_send(chan: Option<&mut mpsc::Sender<f32>>, msg: f32) {
    if let Some(c) = chan {
        let _ = c.try_send(msg);
    }
}

pub(crate) const fn progress(pos: u64, img_size: u64) -> f32 {
    pos as f32 / img_size as f32
}

pub(crate) fn check_token(cancel: Option<&tokio_util::sync::CancellationToken>) -> Result<()> {
    match cancel {
        Some(x) if x.is_cancelled() => Err(crate::Error::Aborted),
        _ => Ok(()),
    }
}

pub(crate) trait Eject {
    fn eject(self) -> io::Result<()>;
}

const BLOCK_SIZE: usize = 4096;

#[derive(Debug)]
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
}

impl<F> DeviceWrapper<F>
where
    F: io::Seek,
{
    pub(crate) fn new(mut f: F) -> io::Result<Self> {
        f.seek(io::SeekFrom::Start(0))?;
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

#[repr(align(512))]
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

impl<W> SdCardWrapper<W>
where
    W: io::Read + io::Write + io::Seek,
{
    pub(crate) fn new(inner: W) -> Self {
        Self {
            inner,
            buf: Box::new(DirectIoBuffer::new()),
            pos: 0,
        }
    }

    fn finish(&mut self) -> io::Result<()> {
        self.inner.seek(io::SeekFrom::Start(0))?;
        self.inner.write_all(self.buf.as_slice())?;
        self.pos = u64::try_from(self.buf.len()).unwrap();

        Ok(())
    }
}

impl<W> Eject for SdCardWrapper<W>
where
    W: io::Read + io::Write + io::Seek + Eject,
{
    fn eject(mut self) -> io::Result<()> {
        self.finish()?;
        self.inner.eject()
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

    fn test_file() -> super::DeviceWrapper<std::io::Cursor<Box<[u8]>>> {
        let data: Vec<u8> = (0..FILE_LEN)
            .map(|x| x % 255)
            .map(|x| u8::try_from(x).unwrap())
            .collect();
        super::DeviceWrapper::new(std::io::Cursor::new(data.into())).unwrap()
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
        let mut temp_buf = vec![0; FILE_LEN].into_boxed_slice();
        let mut sd = SdCardWrapper::new(std::io::Cursor::new(temp_buf.clone()));

        std::io::copy(&mut test_data, &mut sd).unwrap();

        assert_eq!(
            test_data.get_ref()[BLOCK_SIZE..],
            sd.inner.get_ref()[BLOCK_SIZE..]
        );
        assert_eq!(
            test_data.get_ref()[..BLOCK_SIZE],
            sd.buf.as_slice()[..BLOCK_SIZE]
        );
        assert!(sd.inner.get_ref()[..BLOCK_SIZE].iter().all(|x| *x == 0));

        sd.seek(std::io::SeekFrom::Start(0)).unwrap();
        sd.read_exact(&mut temp_buf).unwrap();
        assert_eq!(temp_buf, test_data.get_ref().clone());

        sd.finish().unwrap();
        assert_eq!(test_data.get_ref(), sd.inner.get_ref());
    }
}
