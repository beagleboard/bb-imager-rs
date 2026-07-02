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
