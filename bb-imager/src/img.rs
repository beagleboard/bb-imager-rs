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
    #[error("Zip Error: {0}")]
    ZipError(#[from] zip::result::ZipError),
    #[error("Zip file require inner path")]
    ZipPathError,
    #[error("Zip file sha256 error")]
    ZipSha256Error,
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
        chan: &std::sync::mpsc::Sender<crate::DownloadFlashingStatus>,
    ) -> Result<Self> {
        match img {
            crate::SelectedImage::Local(x) => {
                tokio::task::block_in_place(move || Self::from_path(&x, None))
            }
            crate::SelectedImage::Remote {
                url,
                extract_sha256,
                extract_path,
                ..
            } => {
                let p = downloader
                    .download_progress(url, extract_sha256, chan)
                    .await?;
                tokio::task::block_in_place(move || Self::from_path(&p, extract_path.as_deref()))
            }
        }
    }

    pub fn from_path(path: &Path, inner_path: Option<&str>) -> Result<Self> {
        let mut file = std::fs::File::open(path)?;

        let mut magic = [0u8; 6];
        file.read_exact(&mut magic)?;

        file.seek(std::io::SeekFrom::Start(0))?;

        match magic {
            [0x50, 0x4b, 0x03, 0x04, _, _] => {
                Self::from_zip(file, inner_path.ok_or(Error::ZipPathError)?)
            }
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

    /// TODO: Find way to extract Zipfile with Send
    pub fn from_zip(file: std::fs::File, inner_path: &str) -> Result<Self> {
        let mut res = Vec::new();
        zip::ZipArchive::new(file)
            .map_err(Error::from)?
            .by_name(inner_path)
            .map_err(Error::from)?
            .read_to_end(&mut res)?;

        let hasher = Sha256::new();

        Ok(Self {
            hasher,
            size: res.len() as u64,
            img: OsImageReader::Memory(std::io::Cursor::new(res)),
        })
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

fn size(file: &std::fs::Metadata) -> u64 {
    cfg_if::cfg_if! {
        if #[cfg(unix)] {
            use std::os::unix::fs::MetadataExt;

            file.size()
        } else if #[cfg(windows)] {
            use std::os::windows::fs::MetadataExt;

            file.file_size()
        } else {
            panic!("Unsupported Platform")
        }
    }
}
