use std::{
    io,
    path::Path,
    pin::Pin,
    task::{Context, Poll},
};

use tokio::io::{AsyncSeekExt, AsyncWriteExt};

pub(crate) struct AsyncTempFile(tokio::fs::File);

impl AsyncTempFile {
    pub(crate) fn new() -> io::Result<Self> {
        let f = tempfile::tempfile()?;
        Ok(Self(tokio::fs::File::from_std(f)))
    }

    pub(crate) async fn persist(&mut self, path: &Path) -> io::Result<()> {
        let mut f = tokio::fs::File::create(path).await?;
        self.0.rewind().await?;

        tokio::io::copy(&mut self.0, &mut f).await?;

        // Causes errors if not present
        f.flush().await?;

        Ok(())
    }
}

impl tokio::io::AsyncWrite for AsyncTempFile {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.0).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;

    #[tokio::test]
    async fn test_new_and_write() -> io::Result<()> {
        // 1. Create a new async temp file
        let mut temp_file = AsyncTempFile::new()?;

        // 2. Test AsyncWrite implementation
        let data = b"Hello, Tokio async world!";
        temp_file.write_all(data).await?;
        temp_file.flush().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_persist() -> io::Result<()> {
        let mut temp_file = AsyncTempFile::new()?;
        let data = b"Persisted content data.";

        // Write data to the temporary storage
        temp_file.write_all(data).await?;
        temp_file.flush().await?;

        // Create a named temporary directory to house our persisted file safely
        let target_dir = tempfile::tempdir()?;
        let target_path = target_dir.path().join("persisted_output.txt");

        // 3. Persist the file to the target path
        temp_file.persist(&target_path).await?;

        // 4. Verify the target file contains the exact data
        assert!(target_path.exists(), "Target file was not created");

        let mut verified_file = tokio::fs::File::open(&target_path).await?;
        let mut contents = Vec::new();
        verified_file.read_to_end(&mut contents).await?;

        assert_eq!(contents, data);

        Ok(())
    }
}
