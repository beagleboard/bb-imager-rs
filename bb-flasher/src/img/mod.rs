//! Module to handle extraction of compressed firmware, auto detection of type of extraction, etc

use bb_flasher_sd::ContentType;
#[cfg(feature = "piped_image")]
use bb_helper::file_stream::ReaderFileStream;
use bb_helper::reader_progress::ReaderWithProgress;
use rc_zip_sync::ReadZipStreaming;
use std::{
    io::{self, Read, Seek, SeekFrom},
    path::Path,
    sync::mpsc,
};
#[cfg(feature = "piped_image")]
use tokio_util::task::AbortOnDropHandle;

#[cfg(test)]
mod test;

const XZ_MAGIC: [u8; 6] = [0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00];

pub struct OsArchive {
    inner: OsArchiveCompression,
}

impl OsArchive {
    fn new(img: OsImageSource, chan: Option<mpsc::SyncSender<f32>>, size: u64) -> io::Result<Self> {
        let img = ReaderWithProgress::new(img, size, chan);
        let img = OsArchiveCompression::new(img)?;
        Ok(Self { inner: img })
    }

    pub fn from_path(path: &Path, chan: Option<mpsc::SyncSender<f32>>) -> io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let len = file.metadata()?.len();

        let img = OsImageSource::from(file);
        Self::new(img, chan, len)
    }

    #[cfg(feature = "piped_image")]
    pub fn from_piped(
        img: ReaderFileStream,
        _background: AbortOnDropHandle<io::Result<()>>,
        size: u64,
        chan: Option<mpsc::SyncSender<f32>>,
    ) -> io::Result<Self> {
        let img = OsImageSource::FileStream {
            reader: img,
            _background,
        };
        Self::new(img, chan, size)
    }
}

impl<'a> IntoIterator for &'a mut OsArchive {
    type Item = (Box<str>, ContentType<'a>);
    type IntoIter = Box<dyn Iterator<Item = Self::Item> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        match &mut self.inner {
            OsArchiveCompression::TarXz(archive) => {
                Box::new(archive.entries().unwrap().flat_map(flat_map_with_log))
            }
            OsArchiveCompression::Tar(archive) => {
                Box::new(archive.entries().unwrap().flat_map(flat_map_with_log))
            }
        }
    }
}

fn flat_map_with_log<'a, R: Read>(
    entry: io::Result<tar::Entry<'a, R>>,
) -> Option<(Box<str>, ContentType<'a>)> {
    match entry {
        Ok(x) => Some(tar_entry_map(x)),
        Err(e) => {
            tracing::warn!("Dropping archive entry: {}", e);
            None
        }
    }
}

fn tar_entry_map<'a, R: Read>(entry: tar::Entry<'a, R>) -> (Box<str>, ContentType<'a>) {
    let p = entry.path().unwrap().to_string_lossy().to_string().into();
    let f = if entry.header().entry_type().is_dir() {
        ContentType::Dir
    } else {
        let temp: Box<dyn Read + 'a> = Box::new(entry);
        ContentType::Reader(temp)
    };

    (p, f)
}

type ProgressSource = ReaderWithProgress<OsImageSource>;

enum OsArchiveCompression {
    TarXz(tar::Archive<liblzma::read::XzDecoder<ProgressSource>>),
    Tar(tar::Archive<io::BufReader<ProgressSource>>),
}

impl OsArchiveCompression {
    fn new(mut img: ProgressSource) -> io::Result<Self> {
        let mut magic = [0u8; 6];
        img.read_exact(&mut magic)?;
        img.rewind()?;

        match magic {
            XZ_MAGIC => Ok(Self::TarXz(tar::Archive::new(
                liblzma::read::XzDecoder::new_parallel(img),
            ))),
            _ => Ok(Self::Tar(tar::Archive::new(io::BufReader::new(img)))),
        }
    }
}

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
                #[cfg(feature = "piped_image")]
                OsImageSource::FileStream { .. } => unreachable!(),
            },
        };

        Ok(Self { size, img })
    }

    #[cfg(feature = "piped_image")]
    pub fn from_piped(
        img: ReaderFileStream,
        _background: AbortOnDropHandle<io::Result<()>>,
        size: u64,
    ) -> io::Result<Self> {
        Ok(Self {
            size,
            img: OsImageCompression::new(OsImageSource::FileStream {
                reader: img,
                _background,
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
    #[cfg(feature = "piped_image")]
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
            #[cfg(feature = "piped_image")]
            OsImageSource::FileStream { reader, .. } => reader.read(buf),
        }
    }
}

impl Seek for OsImageSource {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match self {
            OsImageSource::File(file) => file.seek(pos),
            #[cfg(feature = "piped_image")]
            OsImageSource::FileStream { reader, .. } => reader.seek(pos),
        }
    }
}
