//! Module to handle extraction of compressed firmware, auto detection of type of extraction, etc

use bb_helper::file_stream::ReaderFileStream;
use futures::io::AllowStdIo;
use rc_zip_tokio::ReadZipStreaming;
use std::{
    io::{self, SeekFrom},
    num::NonZeroU32,
    path::Path,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt, ReadBuf};
use tokio_util::{compat::FuturesAsyncReadCompatExt, task::AbortOnDropHandle};

#[cfg(test)]
mod test;

const XZ_MAGIC: [u8; 6] = [0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00];

pub struct OsImage {
    size: u64,
    img: OsImageCompression<OsImageSource>,
}

impl OsImage {
    pub async fn from_path(path: &Path) -> io::Result<Self> {
        let file = tokio::fs::File::open(path).await?;
        let mut img = OsImageCompression::new(OsImageSource::from(file)).await?;

        let size = match &mut img {
            OsImageCompression::Xz(_) => {
                let p = path.to_owned();
                tokio::task::spawn_blocking(move || {
                    let file = std::fs::File::open(p)?;
                    liblzma::uncompressed_size(file)
                })
                .await
                .unwrap()?
            }
            OsImageCompression::Zip(x) => x.entry().uncompressed_size,
            OsImageCompression::Uncompressed(x) => match x.get_ref() {
                OsImageSource::File(file) => file.metadata().await?.len(),
                OsImageSource::FileStream { .. } => unreachable!(),
            },
        };

        Ok(Self { size, img })
    }

    pub async fn from_piped(
        img: ReaderFileStream,
        abort_handle: tokio::task::JoinHandle<io::Result<()>>,
        size: u64,
    ) -> io::Result<Self> {
        Ok(Self {
            size,
            img: OsImageCompression::new(OsImageSource::FileStream {
                reader: AllowStdIo::new(img).compat(),
                _background: AbortOnDropHandle::new(abort_handle),
            })
            .await?,
        })
    }

    pub(crate) const fn size(&self) -> u64 {
        self.size
    }
}

impl AsyncRead for OsImage {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.img).poll_read(cx, buf)
    }
}

type TokioAllowStdIo<T> = tokio_util::compat::Compat<AllowStdIo<T>>;

#[allow(clippy::large_enum_variant)]
enum OsImageCompression<I: AsyncRead> {
    Xz(async_compression::tokio::bufread::XzDecoder<tokio::io::BufReader<I>>),
    Zip(rc_zip_tokio::StreamingEntryReader<I>),
    Uncompressed(tokio::io::BufReader<I>),
}

impl<I: AsyncRead + AsyncSeek + Unpin> OsImageCompression<I> {
    async fn new(mut img: I) -> io::Result<Self> {
        let mut magic = [0u8; 6];
        img.read_exact(&mut magic).await?;
        img.rewind().await?;

        match magic {
            XZ_MAGIC => Ok(Self::Xz(
                async_compression::tokio::bufread::XzDecoder::parallel(
                    tokio::io::BufReader::new(img),
                    NonZeroU32::new(2).unwrap(),
                ),
            )),
            [0x50, 0x4b, 0x03, 0x04, _, _] => img
                .stream_zip_entries_throwing_caution_to_the_wind()
                .await
                .map(Self::Zip)
                .map_err(Into::into),
            _ => Ok(Self::Uncompressed(tokio::io::BufReader::new(img))),
        }
    }
}

impl<I> AsyncRead for OsImageCompression<I>
where
    I: AsyncRead + Unpin,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            OsImageCompression::Xz(x) => Pin::new(x).poll_read(cx, buf),
            OsImageCompression::Zip(x) => Pin::new(x).poll_read(cx, buf),
            OsImageCompression::Uncompressed(x) => Pin::new(x).poll_read(cx, buf),
        }
    }
}

enum OsImageSource {
    File(tokio::fs::File),
    FileStream {
        reader: TokioAllowStdIo<ReaderFileStream>,
        _background: AbortOnDropHandle<io::Result<()>>,
    },
}

impl From<tokio::fs::File> for OsImageSource {
    fn from(value: tokio::fs::File) -> Self {
        Self::File(value)
    }
}

impl AsyncRead for OsImageSource {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            OsImageSource::File(x) => Pin::new(x).poll_read(cx, buf),
            OsImageSource::FileStream { reader, .. } => Pin::new(reader).poll_read(cx, buf),
        }
    }
}

impl AsyncSeek for OsImageSource {
    fn start_seek(self: Pin<&mut Self>, position: SeekFrom) -> io::Result<()> {
        match self.get_mut() {
            OsImageSource::File(x) => Pin::new(x).start_seek(position),
            OsImageSource::FileStream { reader, .. } => Pin::new(reader).start_seek(position),
        }
    }

    fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
        match self.get_mut() {
            OsImageSource::File(x) => Pin::new(x).poll_complete(cx),
            OsImageSource::FileStream { reader, .. } => Pin::new(reader).poll_complete(cx),
        }
    }
}
