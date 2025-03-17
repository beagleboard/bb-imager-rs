//! A simple downloader library designed to be used in Applications with support to cache
//! downloaded assets.
//!
//! # Features
//! 
//! - Async
//! - Cache downloaded file in a directory in filesystem.
//! - Check if a file is available in cache.
//! - Uses SHA256 for verifying cached files.
//! - Optional support to download files without caching.

use futures::{Stream, StreamExt, channel::mpsc};
#[cfg(feature = "json")]
use serde::de::DeserializeOwned;
use sha2::{Digest as _, Sha256};
use std::{
    io,
    path::{Path, PathBuf},
    time::Duration,
};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Error, Debug)]
/// Errors for this crate
pub enum Error {
    /// Incorrect Sha256. File might be corrupted
    #[error("Incorrect Sha256. File might be corrupted")]
    Sha256Error,
    #[error("Reqwest Error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Simple downloader that caches files in the provided directory. Uses SHA256 to determine if the
/// file is already downloaded.
///
/// You do not have to wrap the Client in an Rc or Arc to reuse it, because it already uses an Arc
/// internally.
#[derive(Debug, Clone)]
pub struct Downloader {
    client: reqwest::Client,
    cache_dir: PathBuf,
}

impl Downloader {
    /// Create a new downloader that uses a directory for storing cached files.
    pub fn new(cache_dir: PathBuf) -> Self {
        assert!(!cache_dir.is_dir());

        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("Unsupported OS");

        Self { client, cache_dir }
    }

    /// Check if a downloaded file with a particular SHA256 is already in cache.
    pub fn check_cache_from_sha(self, sha256: [u8; 32]) -> Option<PathBuf> {
        let file_path = self.path_from_sha(sha256);

        if file_path.exists() {
            Some(file_path)
        } else {
            None
        }
    }

    /// Check if a downloaded file is already in cache.
    ///
    /// [`check_cache_from_sha`](Self::check_cache_from_sha) should be prefered in cases when SHA256
    /// of the file to download is already known.
    pub fn check_cache_from_url(self, url: &url::Url) -> Option<PathBuf> {
        // Use hash of url for file name
        let file_path = self.path_from_url(url);
        if file_path.exists() {
            Some(file_path)
        } else {
            None
        }
    }

    /// Download a JSON file without caching the contents. Should be used when there is no point in
    /// caching the file.
    #[cfg(feature = "json")]
    pub async fn download_json_no_cache<T, U>(self, url: U) -> Result<T>
    where
        T: DeserializeOwned,
        U: reqwest::IntoUrl,
    {
        self.client
            .get(url)
            .send()
            .await
            .map_err(Error::from)?
            .json()
            .await
            .map_err(Error::from)
    }

    /// Checks if the file is present in cache. If the file is present, returns path to it. Else
    /// downloads the file.
    ///
    /// [`download_with_sha`](Self::download_with_sha) should be prefered when the SHA256 of the
    /// file is known in advance.
    ///
    /// # Progress
    ///
    /// Download progress can be optionally tracked using a [`futures::channel::mpsc`].
    pub async fn download(
        self,
        url: url::Url,
        mut chan: Option<mpsc::Sender<f32>>,
    ) -> Result<PathBuf> {
        // Use hash of url for file name
        let file_path = self.path_from_url(&url);

        if file_path.exists() {
            return Ok(file_path);
        }
        chan_send(chan.as_mut(), 0.0);

        let mut cur_pos = 0;
        let mut tmp_file = AsyncTempFile::new()?;
        {
            let mut tmp_file = tokio::io::BufWriter::new(tmp_file.as_mut());

            let response = self.client.get(url).send().await.map_err(Error::from)?;
            let response_size = response.content_length();
            let mut response_stream = response.bytes_stream();

            let response_size = match response_size {
                Some(x) => x as usize,
                None => response_stream.size_hint().0,
            };

            while let Some(x) = response_stream.next().await {
                let mut data = x.map_err(Error::from)?;
                cur_pos += data.len();
                tmp_file.write_all_buf(&mut data).await?;
                chan_send(chan.as_mut(), (cur_pos as f32) / (response_size as f32));
            }
        }

        tmp_file.persist(&file_path).await?;
        Ok(file_path)
    }

    /// Checks if the file is present in cache. If the file is present, returns path to it. Else
    /// downloads the file.
    ///
    /// Uses SHA256 to verify that the file in cache is valid.
    ///
    /// # Progress
    ///
    /// Download progress can be optionally tracked using a [`futures::channel::mpsc`].
    pub async fn download_with_sha(
        &self,
        url: url::Url,
        sha256: [u8; 32],
        mut chan: Option<mpsc::Sender<f32>>,
    ) -> Result<PathBuf> {
        let file_path = self.path_from_sha(sha256);

        if file_path.exists() {
            let hash = sha256_from_path(&file_path).await?;
            if hash == sha256 {
                return Ok(file_path);
            }

            // Delete old file
            let _ = tokio::fs::remove_file(&file_path).await;
        }
        chan_send(chan.as_mut(), 0.0);

        let mut tmp_file = AsyncTempFile::new()?;
        {
            let mut tmp_file = tokio::io::BufWriter::new(tmp_file.as_mut());

            let response = self.client.get(url).send().await.map_err(Error::from)?;

            let mut cur_pos = 0;
            let response_size = response.content_length();

            let mut response_stream = response.bytes_stream();

            let response_size = match response_size {
                Some(x) => x as usize,
                None => response_stream.size_hint().0,
            };

            let mut hasher = Sha256::new();

            while let Some(x) = response_stream.next().await {
                let mut data = x.map_err(Error::from)?;
                cur_pos += data.len();
                hasher.update(&data);
                tmp_file.write_all_buf(&mut data).await?;

                chan_send(chan.as_mut(), (cur_pos as f32) / (response_size as f32));
            }

            let hash: [u8; 32] = hasher
                .finalize()
                .as_slice()
                .try_into()
                .expect("SHA-256 is 32 bytes");

            if hash != sha256 {
                tracing::warn!("{hash:?} != {sha256:?}");
                return Err(Error::Sha256Error.into());
            }
        }

        tmp_file.persist(&file_path).await?;

        Ok(file_path)
    }

    fn path_from_url(&self, url: &url::Url) -> PathBuf {
        let file_name: [u8; 32] = Sha256::new()
            .chain_update(url.as_str())
            .finalize()
            .as_slice()
            .try_into()
            .expect("SHA-256 is 32 bytes");
        self.path_from_sha(file_name)
    }

    fn path_from_sha(&self, sha256: [u8; 32]) -> PathBuf {
        let file_name = const_hex::encode(sha256);
        self.cache_dir.join(file_name)
    }
}

struct AsyncTempFile(tokio::fs::File);

impl AsyncTempFile {
    fn new() -> std::io::Result<Self> {
        let f = tempfile::tempfile()?;
        Ok(Self(tokio::fs::File::from_std(f)))
    }

    async fn persist(&mut self, path: &Path) -> std::io::Result<()> {
        let mut f = tokio::fs::File::create_new(path).await?;
        self.0.seek(io::SeekFrom::Start(0)).await?;
        tokio::io::copy(&mut self.0, &mut f).await?;
        Ok(())
    }
}

impl AsMut<tokio::fs::File> for AsyncTempFile {
    fn as_mut(&mut self) -> &mut tokio::fs::File {
        &mut self.0
    }
}

async fn sha256_from_path(p: &Path) -> std::io::Result<[u8; 32]> {
    let file = tokio::fs::File::open(p).await?;
    let mut reader = tokio::io::BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0; 512];

    loop {
        let count = reader.read(&mut buffer).await?;
        if count == 0 {
            break;
        }

        hasher.update(&buffer[..count]);
    }

    let hash = hasher
        .finalize()
        .as_slice()
        .try_into()
        .expect("SHA-256 is 32 bytes");

    Ok(hash)
}

fn chan_send(chan: Option<&mut mpsc::Sender<f32>>, msg: f32) {
    if let Some(c) = chan {
        let _ = c.try_send(msg);
    }
}
