//! Module to handle extraction of compressed firmware, auto detection of type of extraction, etc

use bb_helper::file_stream::ReaderFileStream;
use std::{
    io::{Read, Seek, SeekFrom},
    path::Path,
};

pub struct OsImage {
    size: u64,
    img: OsImageReader,
}

pub(crate) enum OsImageReader {
    Xz(liblzma::read::XzDecoder<std::fs::File>),
    XzPiped(liblzma::read::XzDecoder<ReaderFileStream>),
    Uncompressed(std::io::BufReader<std::fs::File>),
    UncompressedPiped(std::io::BufReader<ReaderFileStream>),
}

impl OsImage {
    pub fn from_path(path: &Path) -> std::io::Result<Self> {
        let mut file = std::fs::File::open(path)?;

        let mut magic = [0u8; 6];
        file.read_exact(&mut magic)?;

        file.seek(std::io::SeekFrom::Start(0))?;

        match magic {
            [0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00] => {
                let size = liblzma::uncompressed_size(&mut file)?;

                file.seek(std::io::SeekFrom::Start(0))?;
                let img = liblzma::read::XzDecoder::new_parallel(file);

                Ok(Self {
                    size,
                    img: OsImageReader::Xz(img),
                })
            }
            _ => {
                let size = size(&file.metadata()?);

                Ok(Self {
                    size,
                    img: OsImageReader::Uncompressed(std::io::BufReader::new(file)),
                })
            }
        }
    }

    pub fn from_piped(mut img: ReaderFileStream, size: u64) -> std::io::Result<Self> {
        let mut magic = [0u8; 6];
        img.read_exact(&mut magic)?;
        img.seek(SeekFrom::Start(0))?;

        match magic {
            [0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00] => Ok(Self {
                size,
                img: OsImageReader::XzPiped(liblzma::read::XzDecoder::new_parallel(img)),
            }),
            _ => Ok(Self {
                size,
                img: OsImageReader::UncompressedPiped(std::io::BufReader::new(img)),
            }),
        }
    }

    pub(crate) const fn size(&self) -> u64 {
        self.size
    }
}

impl std::io::Read for OsImage {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match &mut self.img {
            OsImageReader::Xz(x) => x.read(buf),
            OsImageReader::Uncompressed(x) => x.read(buf),
            OsImageReader::XzPiped(x) => x.read(buf),
            OsImageReader::UncompressedPiped(x) => x.read(buf),
        }
    }
}

#[cfg(unix)]
fn size(file: &std::fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    file.size()
}

#[cfg(windows)]
fn size(file: &std::fs::Metadata) -> u64 {
    use std::os::windows::fs::MetadataExt;
    file.file_size()
}
