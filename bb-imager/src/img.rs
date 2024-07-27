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
    sha256: Option<[u8; 32]>,
    img: OsImageReader,
}

pub enum OsImageReader {
    Xz(xz2::read::XzDecoder<std::fs::File>),
    Uncompressed(std::fs::File),
    Memory(std::io::Cursor<Vec<u8>>),
}

impl OsImage {
    pub fn from_path(
        path: &Path,
        inner_path: Option<&str>,
        sha256: Option<[u8; 32]>,
    ) -> Result<Self> {
        let mut file = std::fs::File::open(path)?;

        let mut magic = [0u8; 6];
        file.read_exact(&mut magic)?;

        file.seek(std::io::SeekFrom::Start(0))?;

        let temp = match magic {
            [0x50, 0x4b, 0x03, 0x04, _, _] => OsImageReader::Memory(Self::from_zip(
                file,
                inner_path.ok_or(Error::ZipPathError)?,
                sha256,
            )?),
            [0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00] => {
                OsImageReader::Xz(xz2::read::XzDecoder::new(file))
            }
            _ => OsImageReader::Uncompressed(file),
        };

        Ok(Self { sha256, img: temp })
    }

    /// TODO: Find way to extract Zipfile with Send
    pub fn from_zip(
        file: std::fs::File,
        inner_path: &str,
        sha256: Option<[u8; 32]>,
    ) -> Result<std::io::Cursor<Vec<u8>>> {
        let mut res = Vec::new();
        zip::ZipArchive::new(file)
            .map_err(Error::from)?
            .by_name(inner_path)
            .map_err(Error::from)?
            .read_to_end(&mut res)?;

        if let Some(x) = sha256 {
            let mut hasher = Sha256::new();
            hasher.update(&res);
            let hash: [u8; 32] = hasher
                .finalize()
                .as_slice()
                .try_into()
                .expect("SHA-256 is 32 bytes");

            if hash != x {
                return Err(Error::ZipSha256Error).map_err(Into::into);
            }
        }

        Ok(std::io::Cursor::new(res))
    }

    pub fn sha256(&self) -> Option<[u8; 32]> {
        self.sha256
    }
}

impl std::io::Read for OsImage {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.img.read(buf)
    }
}

impl std::io::Read for OsImageReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            OsImageReader::Memory(x) => x.read(buf),
            OsImageReader::Xz(x) => x.read(buf),
            OsImageReader::Uncompressed(x) => x.read(buf),
        }
    }
}
