//! A data stream with sync Read and async Write halves. Has a backing file.
//!
//! This is designed to be used for large data streams that cannot live in memory.

use std::{
    io,
    path::Path,
    sync::{Arc, Condvar, Mutex},
};

use tokio::io::{AsyncSeekExt, AsyncWriteExt};

type SharedState = Arc<(Mutex<bool>, Condvar)>;

/// Asynchronous writer half of a file-backed stream.
///
/// Writes data asynchronously to a temporary file. The data can be persisted
/// to a permanent location using [`persist`](Self::persist).
pub struct WriterFileStream {
    file: tokio::fs::File,
    writing: SharedState,
}

impl WriterFileStream {
    const fn new(file: tokio::fs::File, writing: SharedState) -> Self {
        Self { file, writing }
    }

    /// Persists the written data to a permanent file location.
    ///
    /// Copies all data from the temporary file to the specified path.
    pub async fn persist(&mut self, path: &Path) -> io::Result<()> {
        let mut f = tokio::fs::File::create(path).await?;
        self.file.seek(io::SeekFrom::Start(0)).await?;

        tokio::io::copy(&mut self.file, &mut f).await?;

        // Causes errors if not present
        f.flush().await?;

        Ok(())
    }
}

impl tokio::io::AsyncWrite for WriterFileStream {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, io::Error>> {
        let res = std::pin::Pin::new(&mut self.file).poll_write(cx, buf);
        self.writing.1.notify_all();
        res
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), io::Error>> {
        let res = std::pin::Pin::new(&mut self.file).poll_flush(cx);
        self.writing.1.notify_all();
        res
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), io::Error>> {
        std::pin::Pin::new(&mut self.file).poll_shutdown(cx)
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

impl std::io::Seek for ReaderFileStream {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.file.seek(pos)
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
    let writer = WriterFileStream::new(file.into_file().into(), flag);

    Ok((writer, reader))
}
