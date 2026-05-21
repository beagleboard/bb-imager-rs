use std::{io, pin::Pin, task::Poll};

use tokio::{
    io::{AsyncReadExt, AsyncSeekExt},
    sync::mpsc,
};

pub(crate) fn chan_send(chan: Option<&mut mpsc::Sender<f32>>, msg: f32) {
    if let Some(c) = chan {
        let _ = c.try_send(msg);
    }
}

pub(crate) const fn progress(pos: u64, img_size: u64) -> f32 {
    pos as f32 / img_size as f32
}

pub(crate) trait EjectAsync {
    fn eject(self) -> impl Future<Output = io::Result<()>>;
}

impl EjectAsync for tokio::fs::File {
    async fn eject(self) -> io::Result<()> {
        self.sync_all().await
    }
}

const BLOCK_SIZE: usize = 4096;

#[derive(Debug)]
/// Wrapper to perform aligned read/write operations.
pub(crate) struct DeviceWrapperAsync<F> {
    f: F,
    offset: u64,
    buf: Box<DirectIoBuffer<BLOCK_SIZE>>,
    cache_offset: u64,
    pending_offset: Option<u64>,
}

impl<F> DeviceWrapperAsync<F> {
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

impl<F> DeviceWrapperAsync<F>
where
    F: tokio::io::AsyncSeek + Unpin,
{
    pub(crate) async fn new(mut f: F) -> io::Result<Self> {
        f.rewind().await?;

        Ok(Self {
            f,
            offset: 0,
            // Hack to make reading from 0 working
            cache_offset: 1,
            buf: Box::new(DirectIoBuffer::new()),
            pending_offset: None,
        })
    }
}

impl<F> DeviceWrapperAsync<F>
where
    F: tokio::io::AsyncRead + tokio::io::AsyncSeek + Unpin,
{
    async fn fill_cache(&mut self) -> io::Result<()> {
        if self.cache_offset != self.block_offset() {
            self.cache_offset = self.block_offset();

            self.f.seek(io::SeekFrom::Start(self.cache_offset)).await?;

            self.f.read_exact(self.buf.as_mut_slice()).await?;
        }

        Ok(())
    }
}

impl<F> tokio::io::AsyncRead for DeviceWrapperAsync<F>
where
    F: tokio::io::AsyncRead + tokio::io::AsyncSeek + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        {
            let fut = self.fill_cache();

            tokio::pin!(fut);

            match fut.poll(cx) {
                Poll::Ready(Ok(())) => {}
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }

        let count = std::cmp::min(buf.remaining(), self.cache_buf_hit_len());

        let start = self.cache_buf_offset();

        buf.put_slice(&self.buf.as_slice()[start..(start + count)]);

        self.offset += count as u64;

        Poll::Ready(Ok(()))
    }
}

impl<F> tokio::io::AsyncWrite for DeviceWrapperAsync<F>
where
    F: tokio::io::AsyncWrite + tokio::io::AsyncRead + tokio::io::AsyncSeek + Unpin,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        {
            let fut = self.fill_cache();
            tokio::pin!(fut);
            match fut.poll(cx) {
                Poll::Ready(Ok(())) => {}
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }

        let count = std::cmp::min(buf.len(), self.cache_buf_hit_len());
        let start = self.cache_buf_offset();

        self.buf.as_mut_slice()[start..(start + count)].copy_from_slice(&buf[..count]);

        {
            let cache_offset = self.cache_offset;
            let (f, write_buf) = {
                let this = &mut *self;
                (&mut this.f, this.buf.as_slice())
            };

            let mut inner = Pin::new(f);

            match inner.as_mut().start_seek(io::SeekFrom::Start(cache_offset)) {
                Ok(()) => {}
                Err(e) => return Poll::Ready(Err(e)),
            }

            match inner.as_mut().poll_complete(cx) {
                Poll::Ready(Ok(_)) => {}
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }

            match inner.poll_write(cx, write_buf) {
                Poll::Ready(Ok(_)) => {}
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }

        self.offset += count as u64;

        Poll::Ready(Ok(count))
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.f).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.f).poll_shutdown(cx)
    }
}

impl<F> tokio::io::AsyncSeek for DeviceWrapperAsync<F>
where
    F: tokio::io::AsyncSeek + Unpin,
{
    fn start_seek(mut self: Pin<&mut Self>, position: io::SeekFrom) -> io::Result<()> {
        let new_offset = match position {
            io::SeekFrom::Start(i) => i,

            io::SeekFrom::Current(i) => self
                .offset
                .checked_add_signed(i)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid seek"))?,

            io::SeekFrom::End(_) => {
                Pin::new(&mut self.f).start_seek(position)?;
                self.pending_offset = None;
                return Ok(());
            }
        };

        self.pending_offset = Some(new_offset);

        Ok(())
    }

    fn poll_complete(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<io::Result<u64>> {
        if let Some(offset) = self.pending_offset.take() {
            self.offset = offset;
            return Poll::Ready(Ok(offset));
        }

        match Pin::new(&mut self.f).poll_complete(cx) {
            Poll::Ready(Ok(pos)) => {
                self.offset = pos;
                Poll::Ready(Ok(pos))
            }

            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),

            Poll::Pending => Poll::Pending,
        }
    }
}

impl<W> EjectAsync for DeviceWrapperAsync<W>
where
    W: tokio::io::AsyncRead + tokio::io::AsyncWrite + tokio::io::AsyncSeek + Unpin + EjectAsync,
{
    async fn eject(self) -> io::Result<()> {
        self.f.eject().await
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
pub(crate) struct SdCardWrapperAsync<W> {
    inner: W,
    buf: Box<DirectIoBuffer<BLOCK_SIZE>>,
    pos: u64,
}

impl<W> SdCardWrapperAsync<W>
where
    W: tokio::io::AsyncRead + tokio::io::AsyncWrite + tokio::io::AsyncSeek + Unpin,
{
    pub(crate) fn new(inner: W) -> Self {
        Self {
            inner,
            buf: Box::new(DirectIoBuffer::new()),
            pos: 0,
        }
    }

    pub(crate) async fn finish(&mut self) -> io::Result<()> {
        use tokio::io::{AsyncSeekExt, AsyncWriteExt};

        self.inner.seek(io::SeekFrom::Start(0)).await?;
        self.inner.write_all(self.buf.as_slice()).await?;
        self.pos = u64::try_from(self.buf.len()).unwrap();

        Ok(())
    }
}

impl<W> EjectAsync for SdCardWrapperAsync<W>
where
    W: tokio::io::AsyncRead + tokio::io::AsyncWrite + tokio::io::AsyncSeek + Unpin + EjectAsync,
{
    async fn eject(mut self) -> io::Result<()> {
        self.finish().await?;
        self.inner.eject().await
    }
}

impl<W> tokio::io::AsyncRead for SdCardWrapperAsync<W>
where
    W: tokio::io::AsyncRead + tokio::io::AsyncSeek + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let pos = usize::try_from(self.pos).unwrap();

        if pos < self.buf.len() {
            let remaining = buf.remaining();
            let count = std::cmp::min(self.buf.len() - pos, remaining);

            let mut inner = Pin::new(&mut self.inner);

            match inner.as_mut().poll_complete(cx) {
                Poll::Ready(Ok(_)) => {}
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }

            if let Err(e) = inner
                .as_mut()
                .start_seek(io::SeekFrom::Current(i64::try_from(count).unwrap()))
            {
                return Poll::Ready(Err(e));
            }
            match inner.poll_complete(cx) {
                Poll::Ready(Ok(_)) => {}
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }

            buf.put_slice(&self.buf.as_slice()[pos..(pos + count)]);
            self.pos += u64::try_from(count).unwrap();

            Poll::Ready(Ok(()))
        } else {
            let before = buf.filled().len();

            match Pin::new(&mut self.inner).poll_read(cx, buf) {
                Poll::Ready(Ok(())) => {
                    let read = buf.filled().len() - before;
                    self.pos += u64::try_from(read).unwrap();
                    Poll::Ready(Ok(()))
                }
                other => other,
            }
        }
    }
}

impl<W> tokio::io::AsyncWrite for SdCardWrapperAsync<W>
where
    W: tokio::io::AsyncWrite + tokio::io::AsyncSeek + Unpin,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let pos = usize::try_from(self.pos).unwrap();

        if pos < self.buf.len() {
            let count = std::cmp::min(self.buf.len() - pos, buf.len());

            let mut inner = Pin::new(&mut self.inner);

            match inner.as_mut().poll_complete(cx) {
                Poll::Ready(Ok(_)) => {}
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }

            if let Err(e) = inner
                .as_mut()
                .start_seek(io::SeekFrom::Current(i64::try_from(count).unwrap()))
            {
                return Poll::Ready(Err(e));
            }
            match inner.poll_complete(cx) {
                Poll::Ready(Ok(_)) => {}
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }

            self.buf.as_mut_slice()[pos..(pos + count)].copy_from_slice(&buf[..count]);

            self.pos += u64::try_from(count).unwrap();

            Poll::Ready(Ok(count))
        } else {
            match Pin::new(&mut self.inner).poll_write(cx, buf) {
                Poll::Ready(Ok(count)) => {
                    self.pos += u64::try_from(count).unwrap();
                    Poll::Ready(Ok(count))
                }
                other => other,
            }
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

impl<W> tokio::io::AsyncSeek for SdCardWrapperAsync<W>
where
    W: tokio::io::AsyncSeek + Unpin,
{
    fn start_seek(mut self: Pin<&mut Self>, position: io::SeekFrom) -> io::Result<()> {
        Pin::new(&mut self.inner).start_seek(position)
    }

    fn poll_complete(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<io::Result<u64>> {
        match Pin::new(&mut self.inner).poll_complete(cx) {
            Poll::Ready(Ok(pos)) => {
                self.pos = pos;
                Poll::Ready(Ok(pos))
            }
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::SeekFrom;

    use tokio::io::AsyncWriteExt;

    use super::*;

    const FILE_LEN: usize = 12 * 1024;

    fn test_data() -> std::io::Cursor<Box<[u8]>> {
        let data: Vec<u8> = (0..FILE_LEN)
            .map(|x| x % 255)
            .map(|x| u8::try_from(x).unwrap())
            .collect();
        std::io::Cursor::new(data.into())
    }

    async fn test_file() -> super::DeviceWrapperAsync<std::io::Cursor<Box<[u8]>>> {
        let data: Vec<u8> = (0..FILE_LEN)
            .map(|x| x % 255)
            .map(|x| u8::try_from(x).unwrap())
            .collect();
        super::DeviceWrapperAsync::new(std::io::Cursor::new(data.into()))
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn dev_wrapper_read() {
        let mut temp = test_file().await;
        let mut buf = [0u8; 50];

        temp.seek(SeekFrom::Start(10)).await.unwrap();
        temp.read_exact(&mut buf).await.unwrap();

        let ans: Vec<u8> = (10..60).collect();
        assert_eq!(buf.as_slice(), &ans);

        temp.seek(SeekFrom::Start(4095)).await.unwrap();
        temp.read_exact(&mut buf).await.unwrap();

        let ans: Vec<u8> = (4095..4145).map(|x| (x % 255) as u8).collect();
        assert_eq!(buf.as_slice(), &ans);
    }

    #[tokio::test]
    async fn dev_wrapper_write() {
        let mut temp = test_file().await;
        let ans = [9u8; 50];

        let mut buf = [9u8; 50];
        temp.seek(SeekFrom::Start(10)).await.unwrap();
        temp.write_all(&buf).await.unwrap();

        temp.seek(SeekFrom::Start(4090)).await.unwrap();
        temp.write_all(&buf).await.unwrap();

        temp.seek(SeekFrom::Start(10)).await.unwrap();
        temp.read_exact(&mut buf).await.unwrap();

        assert_eq!(ans, buf);

        temp.seek(SeekFrom::Start(4090)).await.unwrap();
        temp.read_exact(&mut buf).await.unwrap();

        assert_eq!(ans, buf);
    }

    #[tokio::test]
    async fn sd_card_wrapper() {
        let mut test_data = test_data();
        let mut temp_buf = vec![0; FILE_LEN].into_boxed_slice();
        let mut sd = SdCardWrapperAsync::new(std::io::Cursor::new(temp_buf.clone()));

        tokio::io::copy(&mut test_data, &mut sd).await.unwrap();

        assert_eq!(
            test_data.get_ref()[BLOCK_SIZE..],
            sd.inner.get_ref()[BLOCK_SIZE..]
        );
        assert_eq!(
            test_data.get_ref()[..BLOCK_SIZE],
            sd.buf.as_slice()[..BLOCK_SIZE]
        );
        assert!(sd.inner.get_ref()[..BLOCK_SIZE].iter().all(|x| *x == 0));

        sd.rewind().await.unwrap();
        sd.read_exact(&mut temp_buf).await.unwrap();
        assert_eq!(temp_buf, test_data.get_ref().clone());

        sd.finish().await.unwrap();
        assert_eq!(test_data.get_ref(), sd.inner.get_ref());
    }
}
