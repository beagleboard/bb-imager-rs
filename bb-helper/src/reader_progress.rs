use std::{io, sync::mpsc};

pub struct ReaderWithProgress<R> {
    reader: R,
    pos: u64,
    size: u64,
    chan: Option<mpsc::SyncSender<f32>>,
}

impl<R> ReaderWithProgress<R> {
    pub const fn new(reader: R, size: u64, chan: Option<mpsc::SyncSender<f32>>) -> Self {
        Self {
            reader,
            size,
            chan,
            pos: 0,
        }
    }
}

impl<R: io::Read> io::Read for ReaderWithProgress<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let count = self.reader.read(buf)?;

        self.pos += count as u64;
        if let Some(tx) = &self.chan
            && self.size != 0
        {
            let _ = tx.try_send(self.pos as f32 / self.size as f32);
        }

        Ok(count)
    }
}

impl<R: io::Seek> io::Seek for ReaderWithProgress<R> {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.pos = self.reader.seek(pos)?;
        Ok(self.pos)
    }
}

