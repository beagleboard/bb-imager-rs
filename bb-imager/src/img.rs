//! Module to handle extraction of compressed firmware, auto detection of type of extraction, etc

use crate::error::Result;
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Seek},
    path::Path,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to read image {0}")]
    FailedToReadImage(String),
}

pub struct OsImage {
    size: u64,
    hasher: Sha256,
    img: OsImageReader,
}

pub enum OsImageReader {
    Xz(liblzma::read::XzDecoder<std::fs::File>),
    Uncompressed(std::io::BufReader<std::fs::File>),
    Memory(std::io::Cursor<Vec<u8>>),
}

impl OsImage {
    pub async fn from_selected_image(
        img: crate::SelectedImage,
        downloader: &crate::download::Downloader,
        chan: &tokio::sync::mpsc::Sender<crate::DownloadFlashingStatus>,
    ) -> Result<Self> {
        match img {
            crate::SelectedImage::Local(x) => {
                tokio::task::block_in_place(move || Self::from_path(&x))
            }
            crate::SelectedImage::Remote {
                url,
                extract_sha256,
                ..
            } => {
                let p = downloader
                    .download_progress(url, extract_sha256, chan)
                    .await?;
                tokio::task::block_in_place(move || Self::from_path(&p))
            }
            _ => panic!("No image selected"),
        }
    }

    pub fn from_path(path: &Path) -> Result<Self> {
        let mut file = std::fs::File::open(path)?;

        let mut magic = [0u8; 6];
        file.read_exact(&mut magic)?;

        file.seek(std::io::SeekFrom::Start(0))?;

        match magic {
            [0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00] => {
                let size = liblzma::uncompressed_size(&mut file)?;

                file.seek(std::io::SeekFrom::Start(0))?;
                let img = liblzma::read::XzDecoder::new_parallel(file);
                let hasher = Sha256::new();

                Ok(Self {
                    hasher,
                    size,
                    img: OsImageReader::Xz(img),
                })
            }
            _ => {
                let size = size(&file.metadata()?);
                let hasher = Sha256::new();

                Ok(Self {
                    hasher,
                    size,
                    img: OsImageReader::Uncompressed(std::io::BufReader::new(file)),
                })
            }
        }
    }

    pub fn sha256(&self) -> [u8; 32] {
        self.hasher
            .clone()
            .finalize()
            .as_slice()
            .try_into()
            .expect("SHA-256 is 32 bytes")
    }

    pub fn size(&self) -> u64 {
        self.size
    }
}

impl std::io::Read for OsImage {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let count = match &mut self.img {
            OsImageReader::Xz(x) => x.read(buf),
            OsImageReader::Uncompressed(x) => x.read(buf),
            OsImageReader::Memory(x) => std::io::Read::read(x, buf),
        }?;

        self.hasher.update(&buf[..count]);
        Ok(count)
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
