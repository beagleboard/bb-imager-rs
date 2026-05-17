//! Module to handle extraction of compressed firmware, auto detection of type of extraction, etc

use bb_helper::file_stream::ReaderFileStream;
use rc_zip_sync::ReadZipStreaming;
use std::{
    io::{self, Read, Seek, SeekFrom},
    path::Path,
};
use tokio_util::task::AbortOnDropHandle;

const XZ_MAGIC: [u8; 6] = [0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00];

pub struct OsImage {
    size: u64,
    img: OsImageCompression<OsImageSource>,
}

impl OsImage {
    pub fn from_path(path: &Path) -> io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let mut img = OsImageCompression::new(OsImageSource::from(file))?;

        let size = match &mut img {
            OsImageCompression::Xz(x) => {
                let size = liblzma::uncompressed_size(x.get_mut())?;
                x.get_mut().rewind()?;
                size
            }
            OsImageCompression::Zip(x) => x.entry().uncompressed_size,
            OsImageCompression::Uncompressed(x) => match x.get_ref() {
                OsImageSource::File(file) => file.metadata()?.len(),
                OsImageSource::FileStream { .. } => unreachable!(),
            },
        };

        Ok(Self { size, img })
    }

    pub fn from_piped(
        img: ReaderFileStream,
        abort_handle: tokio::task::JoinHandle<io::Result<()>>,
        size: u64,
    ) -> io::Result<Self> {
        Ok(Self {
            size,
            img: OsImageCompression::new(OsImageSource::FileStream {
                reader: img,
                _background: AbortOnDropHandle::new(abort_handle),
            })?,
        })
    }

    pub(crate) const fn size(&self) -> u64 {
        self.size
    }
}

impl Read for OsImage {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match &mut self.img {
            OsImageCompression::Xz(x) => x.read(buf),
            OsImageCompression::Zip(x) => x.read(buf),
            OsImageCompression::Uncompressed(x) => x.read(buf),
        }
    }
}

#[allow(clippy::large_enum_variant)]
enum OsImageCompression<I: Read> {
    Xz(liblzma::read::XzDecoder<I>),
    Zip(rc_zip_sync::StreamingEntryReader<I>),
    Uncompressed(io::BufReader<I>),
}

impl<I: Read + Seek> OsImageCompression<I> {
    fn new(mut img: I) -> io::Result<Self> {
        let mut magic = [0u8; 6];
        img.read_exact(&mut magic)?;
        img.rewind()?;

        match magic {
            XZ_MAGIC => Ok(Self::Xz(liblzma::read::XzDecoder::new_parallel(img))),
            [0x50, 0x4b, 0x03, 0x04, _, _] => img
                .stream_zip_entries_throwing_caution_to_the_wind()
                .map(Self::Zip)
                .map_err(Into::into),
            _ => Ok(Self::Uncompressed(std::io::BufReader::new(img))),
        }
    }
}

impl<I: Read> Read for OsImageCompression<I> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            OsImageCompression::Xz(x) => x.read(buf),
            OsImageCompression::Zip(x) => x.read(buf),
            OsImageCompression::Uncompressed(x) => x.read(buf),
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
