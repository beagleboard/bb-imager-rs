//! Module to handle extraction of compressed firmware, auto detection of type of extraction, etc

use crate::{DownloadFlashingStatus, ImageFile};
use futures::channel::mpsc;
use std::{
    io::{Read, Seek},
    path::Path,
};

pub(crate) struct OsImage {
    size: u64,
    img: OsImageReader,
}

pub(crate) enum OsImageReader {
    Xz(liblzma::read::XzDecoder<std::fs::File>),
    Uncompressed(std::io::BufReader<std::fs::File>),
}

impl OsImage {
    pub(crate) async fn open(
        img: impl ImageFile,
        chan: Option<mpsc::Sender<DownloadFlashingStatus>>,
    ) -> std::io::Result<Self> {
        let img_path = img.resolve(chan).await?;
        Self::from_path(&img_path)
    }

    pub(crate) fn from_path(path: &Path) -> std::io::Result<Self> {
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

    pub(crate) const fn size(&self) -> u64 {
        self.size
    }
}

impl std::io::Read for OsImage {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match &mut self.img {
            OsImageReader::Xz(x) => x.read(buf),
            OsImageReader::Uncompressed(x) => x.read(buf),
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
