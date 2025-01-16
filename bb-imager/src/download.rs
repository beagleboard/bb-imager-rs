//! Module for downloading remote images for flashing

use directories::ProjectDirs;
use futures::{Stream, StreamExt};
#[cfg(feature = "config")]
use serde::de::DeserializeOwned;
use sha2::{Digest as _, Sha256};
use std::{
    io,
    path::{Path, PathBuf},
    time::Duration,
};
use thiserror::Error;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

use crate::{error::Result, util::sha256_file_progress, DownloadFlashingStatus};

#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error("UnknownError")]
    UnknownError,
    #[error("Reqwest Error: {0}")]
    ReqwestError(String),
    #[error("Incorrect Sha256. File might be corrupted")]
    Sha256Error,
    #[error("Json Parsing Error: {0}")]
    JsonError(String),
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Self::ReqwestError(value.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct Downloader {
    client: reqwest::Client,
    dirs: ProjectDirs,
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new()
    }
}

impl Downloader {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("Unsupported OS");

        let dirs = ProjectDirs::from("org", "beagleboard", "imagingutility")
            .expect("Failed to find project directories");

        tracing::info!("Cache Dir: {:?}", dirs.cache_dir());
        if let Err(e) = std::fs::create_dir_all(dirs.cache_dir()) {
            if e.kind() != io::ErrorKind::AlreadyExists {
                panic!(
                    "Failed to create cache dir: {:?} due to error {e}",
                    dirs.cache_dir()
                )
            }
        }

        Self { client, dirs }
    }

    pub fn check_cache(self, sha256: [u8; 32]) -> Option<PathBuf> {
        let file_path = self.path_from_sha(sha256);

        if file_path.exists() {
            Some(file_path)
        } else {
            None
        }
    }

    pub fn check_image(self, url: &url::Url) -> Option<PathBuf> {
        // Use hash of url for file name
        let file_path = self.path_from_url(url);
        if file_path.exists() {
            Some(file_path)
        } else {
            None
        }
    }

    #[cfg(feature = "config")]
    pub async fn download_json<T, U>(self, url: U) -> Result<T>
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
            .map_err(|e| Error::JsonError(e.to_string()).into())
    }

    pub async fn download_without_sha(self, url: url::Url) -> Result<PathBuf> {
        // Use hash of url for file name
        let file_path = self.path_from_url(&url);

        if file_path.exists() {
            return Ok(file_path);
        }

        let mut tmp_file = AsyncTempFile::new()?;

        let response = self.client.get(url).send().await.map_err(Error::from)?;

        let mut response_stream = response.bytes_stream();

        while let Some(x) = response_stream.next().await {
            let mut data = x.map_err(Error::from)?;
            tmp_file.as_mut().write_all_buf(&mut data).await?;
        }

        tmp_file.persist(&file_path).await?;
        Ok(file_path)
    }

    pub async fn download_progress(
        &self,
        url: url::Url,
        sha256: [u8; 32],
        chan: &tokio::sync::mpsc::Sender<DownloadFlashingStatus>,
    ) -> Result<PathBuf> {
        let file_path = self.path_from_sha(sha256);

        if file_path.exists() {
            let _ = chan.try_send(DownloadFlashingStatus::VerifyingProgress(0.0));

            let hash = sha256_file_progress(&file_path, chan).await?;
            if hash == sha256 {
                return Ok(file_path);
            }

            // Delete old file
            let _ = tokio::fs::remove_file(&file_path).await;
        }
        let _ = chan.try_send(DownloadFlashingStatus::DownloadingProgress(0.0));

        let mut tmp_file = AsyncTempFile::new()?;
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
            tmp_file.as_mut().write_all_buf(&mut data).await?;

            let _ = chan.try_send(DownloadFlashingStatus::DownloadingProgress(
                (cur_pos as f32) / (response_size as f32),
            ));
        }

        let hash: [u8; 32] = hasher
            .finalize()
            .as_slice()
            .try_into()
            .expect("SHA-256 is 32 bytes");

        let _ = chan.try_send(DownloadFlashingStatus::Verifying);

        if hash != sha256 {
            tracing::warn!("{hash:?} != {sha256:?}");
            return Err(Error::Sha256Error.into());
        }

        tmp_file.persist(&file_path).await?;

        Ok(file_path)
    }

    pub async fn download(self, url: url::Url, sha256: [u8; 32]) -> Result<PathBuf> {
        let (tx, _) = tokio::sync::mpsc::channel(1);
        self.download_progress(url, sha256, &tx).await
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
        self.dirs.cache_dir().join(file_name)
    }
}

struct AsyncTempFile(tokio::fs::File);

impl AsyncTempFile {
    fn new() -> Result<Self> {
        let f = tempfile::tempfile()?;
        Ok(Self(tokio::fs::File::from_std(f)))
    }

    async fn persist(&mut self, path: &Path) -> Result<()> {
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
