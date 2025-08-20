//! Module to handle extraction of compressed firmware, auto detection of type of extraction, etc

use std::{
    io::{Read, Seek},
    path::Path,
};
use tokio::io::AsyncReadExt;

pub struct OsImage {
    size: u64,
    img: OsImageReader,
}

pub type ReadPipe = tokio::io::ReadHalf<tokio::io::SimplexStream>;
type Piped = ReaderWithPrefix<6, tokio_util::io::SyncIoBridge<ReadPipe>>;

pub(crate) enum OsImageReader {
    Xz(liblzma::read::XzDecoder<std::fs::File>),
    XzPiped(liblzma::read::XzDecoder<Piped>),
    Uncompressed(std::io::BufReader<std::fs::File>),
    UncompressedPiped(std::io::BufReader<Piped>),
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

    pub async fn from_piped(mut img: ReadPipe, size: u64) -> std::io::Result<Self> {
        let mut magic = [0u8; 6];
        img.read_exact(&mut magic).await?;

        let img = tokio_util::io::SyncIoBridge::new(img);
        let img = ReaderWithPrefix::new(magic, img);
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

#[derive(Debug)]
pub(crate) struct ReaderWithPrefix<const C: usize, R: Read> {
    prefix: [u8; C],
    reader: R,
    pos: u64,
}

impl<const C: usize, R: Read> ReaderWithPrefix<C, R> {
    fn new(prefix: [u8; C], reader: R) -> Self {
        Self {
            prefix,
            reader,
            pos: 0,
        }
    }
}

impl<const C: usize, R: Read> Read for ReaderWithPrefix<C, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.pos < C as u64 {
            let count = std::cmp::min(buf.len(), C - self.pos as usize);
            let end = self.pos as usize + count;
            buf[..count].copy_from_slice(&self.prefix[(self.pos as usize)..end]);
            self.pos += count as u64;
            Ok(count)
        } else {
            let count = self.reader.read(buf)?;
            self.pos += count as u64;
            Ok(count)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::ReaderWithPrefix;

    #[test]
    fn reader_with_prefix() {
        let prefix = [0, 1, 2, 3, 4, 5];
        let data = [9, 8, 7, 6, 5, 4, 3, 2, 1];

        let mut reader = ReaderWithPrefix::new(prefix, std::io::Cursor::new(data));

        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).unwrap();
        assert_eq!(buf.len(), prefix.len() + data.len());
        assert_eq!(buf[..prefix.len()], prefix);
        assert_eq!(buf[prefix.len()..], data);
    }
}
