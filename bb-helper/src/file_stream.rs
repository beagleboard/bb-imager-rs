//! A data stream with sync Read and async Write halves. Has a backing file.
//!
//! This is designed to be used for large data streams that cannot live in memory.

use std::{
    io::{self, Seek, Write},
    path::Path,
    sync::{Arc, Condvar, Mutex},
};

type SharedState = Arc<(Mutex<bool>, Condvar)>;

/// Asynchronous writer half of a file-backed stream.
///
/// Writes data asynchronously to a temporary file. The data can be persisted
/// to a permanent location using [`persist`](Self::persist).
pub struct WriterFileStream {
    file: std::fs::File,
    writing: SharedState,
}

impl WriterFileStream {
    const fn new(file: std::fs::File, writing: SharedState) -> Self {
        Self { file, writing }
    }

    /// Persists the written data to a permanent file location.
    ///
    /// Copies all data from the temporary file to the specified path.
    pub fn persist(&mut self, path: &Path) -> io::Result<()> {
        let mut f = std::fs::File::create(path)?;
        self.file.seek(io::SeekFrom::Start(0))?;

        std::io::copy(&mut self.file, &mut f)?;

        // Causes errors if not present
        f.flush()
    }
}

impl std::io::Write for WriterFileStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let res = self.file.write(buf)?;
        self.writing.1.notify_all();
        Ok(res)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()?;
        self.writing.1.notify_all();
        Ok(())
    }
}

impl Drop for WriterFileStream {
    fn drop(&mut self) {
        let (lock, cvar) = &*self.writing;
        let mut writing = lock.lock().unwrap();
        *writing = false;
        cvar.notify_all();
    }
}

/// Synchronous reader half of a file-backed stream.
///
/// Reads data from the same temporary file as the writer. While the writer
/// is active, reading will block until data is available or the writer closes.
pub struct ReaderFileStream {
    file: std::fs::File,
    writing: SharedState,
}

impl ReaderFileStream {
    const fn new(file: std::fs::File, writing: SharedState) -> Self {
        Self { file, writing }
    }
}

impl std::io::Read for ReaderFileStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            let count = self.file.read(buf)?;

            if count == 0 {
                let (lock, cvar) = &*self.writing;
                let writing = lock.lock().unwrap();

                if *writing {
                    drop(cvar.wait(writing));
                    continue;
                }
            }

            return Ok(count);
        }
    }
}

// ReaderFileStream behaves like a stream backed by a growing file rather than
// a normal fully materialized file.
//
// Seeking within the currently written region works normally. However:
//
// - Seeking beyond the current file length waits until the writer produces
//   enough data or closes.
// - SeekFrom::End is unsupported while the writer is still active because the
//   final file length is not yet known, so offsets relative to the end cannot
//   be resolved correctly.
// - Once the writer is dropped, seek behavior matches normal file semantics.
impl std::io::Seek for ReaderFileStream {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        loop {
            let (lock, cvar) = &*self.writing;
            let writing = lock.lock().unwrap();

            // If writing done, use normal file seek.
            if !*writing {
                return self.file.seek(pos);
            }

            let len = self.file.metadata()?.len();
            let target = match pos {
                io::SeekFrom::Start(x) => x,
                io::SeekFrom::End(_) => {
                    // We don't know true file len yet. So just return
                    // unsupported. Not sure if we should just wait for writing to finish.
                    return Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        "Seek from end is unsupported",
                    ));
                }
                io::SeekFrom::Current(x) => self
                    .file
                    .stream_position()?
                    .checked_add_signed(x)
                    .ok_or(io::Error::new(io::ErrorKind::InvalidInput, "invalid seek"))?,
            };

            if target <= len {
                return self.file.seek(pos);
            }
            drop(cvar.wait(writing));
        }
    }
}

/// Creates a new file-backed stream with separate reader and writer halves.
///
/// Returns a tuple of (writer, reader) that share a temporary file.
/// The writer can write asynchronously, while the reader provides synchronous access.
pub fn file_stream() -> io::Result<(WriterFileStream, ReaderFileStream)> {
    let file = tempfile::NamedTempFile::new()?;
    let flag = Arc::new((Mutex::new(true), Condvar::new()));

    let reader = ReaderFileStream::new(file.reopen()?, flag.clone());
    let writer = WriterFileStream::new(file.into_file(), flag);

    Ok((writer, reader))
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Seek, SeekFrom};

    use super::*;

    #[test]
    fn seek_from_start_works() {
        let (mut writer, mut reader) = file_stream().unwrap();

        writer.write_all(b"abcdef").unwrap();
        writer.flush().unwrap();

        reader.seek(SeekFrom::Start(2)).unwrap();

        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf).unwrap();

        assert_eq!(&buf, b"cd");
    }

    #[test]
    fn seek_from_current_works() {
        let (mut writer, mut reader) = file_stream().unwrap();

        writer.write_all(b"abcdef").unwrap();
        writer.flush().unwrap();

        reader.seek(SeekFrom::Start(1)).unwrap();
        reader.seek(SeekFrom::Current(2)).unwrap();

        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf).unwrap();

        assert_eq!(&buf, b"d");
    }

    #[test]
    fn seek_from_end_fails_while_writer_alive() {
        let (_writer, mut reader) = file_stream().unwrap();

        let err = reader.seek(SeekFrom::End(0)).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Unsupported);
    }

    #[test]
    fn seek_from_end_works_after_writer_drop() {
        let (mut writer, mut reader) = file_stream().unwrap();

        writer.write_all(b"abcdef").unwrap();
        writer.flush().unwrap();
        drop(writer);

        let pos = reader.seek(SeekFrom::End(-2)).unwrap();
        assert_eq!(pos, 4);

        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf).unwrap();

        assert_eq!(&buf, b"ef");
    }

    #[test]
    fn invalid_negative_seek_fails() {
        let (mut writer, mut reader) = file_stream().unwrap();

        writer.write_all(b"abc").unwrap();
        writer.flush().unwrap();

        let err = reader.seek(SeekFrom::Current(-10)).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }
}
