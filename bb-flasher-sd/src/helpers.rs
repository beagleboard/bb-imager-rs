use std::io;

use futures::channel::mpsc;

use crate::Result;

pub(crate) fn chan_send(chan: Option<&mut mpsc::Sender<f32>>, msg: f32) {
    if let Some(c) = chan {
        let _ = c.try_send(msg);
    }
}

pub(crate) const fn progress(pos: usize, img_size: u64) -> f32 {
    pos as f32 / img_size as f32
}

pub(crate) fn check_arc(cancel: Option<&std::sync::Weak<()>>) -> Result<()> {
    match cancel {
        Some(x) if x.strong_count() == 0 => Err(crate::Error::Aborted),
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
    buf: Box<[u8]>,
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
            buf: vec![0u8; BLOCK_SIZE].into(),
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
            self.f.read_exact(&mut self.buf)
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

        buf[..count]
            .copy_from_slice(&self.buf[self.cache_buf_offset()..(self.cache_buf_offset() + count)]);

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

        self.buf[start..(start + count)].copy_from_slice(&buf[..count]);

        self.f.seek(io::SeekFrom::Start(self.cache_offset))?;
        self.f.write(&self.buf)?;

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

#[cfg(test)]
mod tests {
    use std::io::{Read, Seek, SeekFrom, Write};

    fn test_file() -> super::DeviceWrapper<std::io::Cursor<Box<[u8]>>> {
        const FILE_LEN: usize = 12 * 1024;

        let data: Vec<u8> = (0..FILE_LEN)
            .into_iter()
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

        let ans: Vec<u8> = (10..60).into_iter().collect();
        assert_eq!(buf.as_slice(), &ans);

        temp.seek(SeekFrom::Start(4095)).unwrap();
        temp.read_exact(&mut buf).unwrap();

        let ans: Vec<u8> = (4095..4145).into_iter().map(|x| (x % 255) as u8).collect();
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
}
