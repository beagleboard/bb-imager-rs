//! Module to handle extraction of compressed firmware, auto detection of type of extraction, etc

use bb_helper::file_stream::ReaderFileStream;
use futures::io::AllowStdIo;
use rc_zip_tokio::ReadZipStreaming;
use std::{
    io::{self, Read, Seek, SeekFrom},
    num::NonZeroU32,
    path::Path,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, ReadBuf};
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
        let file = std::fs::File::open(path)?;
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
            OsImageCompression::Uncompressed(x) => match x.get_ref().get_ref().get_ref() {
                OsImageSource::File(file) => file.metadata()?.len(),
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
                reader: img,
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
enum OsImageCompression<I: Read> {
    Xz(async_compression::tokio::bufread::XzDecoder<tokio::io::BufReader<TokioAllowStdIo<I>>>),
    Zip(rc_zip_tokio::StreamingEntryReader<TokioAllowStdIo<I>>),
    Uncompressed(tokio::io::BufReader<TokioAllowStdIo<I>>),
}

impl<I: Read + Seek> OsImageCompression<I> {
    async fn new(mut img: I) -> io::Result<Self> {
        let mut magic = [0u8; 6];
        img.read_exact(&mut magic)?;
        img.rewind()?;

        match magic {
            XZ_MAGIC => Ok(Self::Xz(
                async_compression::tokio::bufread::XzDecoder::parallel(
                    tokio::io::BufReader::new(AllowStdIo::new(img).compat()),
                    NonZeroU32::new(2).unwrap(),
                ),
            )),
            [0x50, 0x4b, 0x03, 0x04, _, _] => AllowStdIo::new(img)
                .compat()
                .stream_zip_entries_throwing_caution_to_the_wind()
                .await
                .map(Self::Zip)
                .map_err(Into::into),
            _ => Ok(Self::Uncompressed(tokio::io::BufReader::new(
                AllowStdIo::new(img).compat(),
            ))),
        }
    }
}

impl<I> AsyncRead for OsImageCompression<I>
where
    I: Read,
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
    File(std::fs::File),
    FileStream {
        reader: ReaderFileStream,
        _background: AbortOnDropHandle<io::Result<()>>,
    },
}

impl From<std::fs::File> for OsImageSource {
    fn from(value: std::fs::File) -> Self {
        Self::File(value)
    }
}

impl Read for OsImageSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            OsImageSource::File(x) => x.read(buf),
            OsImageSource::FileStream { reader, .. } => reader.read(buf),
        }
    }
}

impl Seek for OsImageSource {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match self {
            OsImageSource::File(file) => file.seek(pos),
            OsImageSource::FileStream { reader, .. } => reader.seek(pos),
        }
    }
}
