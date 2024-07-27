//! Module to handle extraction of compressed firmware, auto detection of type of extraction, etc

use crate::error::Result;
use futures_util::{StreamExt, TryStreamExt};
use sha2::{Digest, Sha256};
use std::{io::Read, os::unix::fs::MetadataExt, path::Path};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::bytes::BufMut;

const BUFFER_LEN: usize = 8 * 1024;

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
    size: usize,
    img: OsImageReader,
}

pub enum OsImageReader {
    Xz(async_compression::tokio::bufread::XzDecoder<tokio::io::BufReader<tokio::fs::File>>),
    Uncompressed(tokio::fs::File),
    Memory(std::io::Cursor<Vec<u8>>),
}

impl OsImage {
    pub async fn from_path(
        path: &Path,
        inner_path: Option<&str>,
        sha256: Option<[u8; 32]>,
    ) -> Result<Self> {
        let mut file = tokio::fs::File::open(path).await?;

        let mut magic = [0u8; 6];
        file.read_exact(&mut magic).await?;

        file.seek(std::io::SeekFrom::Start(0)).await?;

        match magic {
            [0x50, 0x4b, 0x03, 0x04, _, _] => {
                let buf = Self::from_zip(
                    file.into_std().await,
                    inner_path.ok_or(Error::ZipPathError)?,
                    sha256,
                )?;

                Ok(Self {
                    sha256,
                    size: buf.len(),
                    img: OsImageReader::Memory(std::io::Cursor::new(buf)),
                })
            }
            [0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00] => {
                let mut img = async_compression::tokio::bufread::XzDecoder::new(
                    tokio::io::BufReader::new(&mut file),
                );

                // TODO: Find something more efficient
                let size = tokio::io::copy(&mut img, &mut tokio::io::empty()).await? as usize;
                drop(img);

                file.seek(std::io::SeekFrom::Start(0)).await?;
                let img = async_compression::tokio::bufread::XzDecoder::new(
                    tokio::io::BufReader::new(file),
                );

                Ok(Self {
                    sha256,
                    size,
                    img: OsImageReader::Xz(img),
                })
            }
            _ => {
                let size = file.metadata().await?.size() as usize;

                Ok(Self {
                    sha256,
                    size,
                    img: OsImageReader::Uncompressed(file),
                })
            }
        }
    }

    /// TODO: Find way to extract Zipfile with Send
    pub fn from_zip(
        file: std::fs::File,
        inner_path: &str,
        sha256: Option<[u8; 32]>,
    ) -> Result<Vec<u8>> {
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

        Ok(res)
    }

    pub fn sha256(&self) -> Option<[u8; 32]> {
        self.sha256
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

impl tokio::io::AsyncRead for OsImage {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match &mut self.img {
            OsImageReader::Xz(ref mut x) => std::pin::Pin::new(x).poll_read(cx, buf),
            OsImageReader::Uncompressed(ref mut x) => std::pin::Pin::new(x).poll_read(cx, buf),
            OsImageReader::Memory(x) => {
                buf.put(x);
                std::task::Poll::Ready(Ok(()))
            }
        }
    }
}
