//! Module for downloading remote images for flashing

use directories::ProjectDirs;
use futures::{SinkExt, Stream, StreamExt};
use sha2::{Digest as _, Sha256};
use std::{
    io,
    path::{Path, PathBuf},
    time::Duration,
};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::io::StreamReader;

use crate::{error::Result, util::sha256_file_progress, DownloadFlashingStatus, BUF_SIZE};

const FILE_NAME_TRIES: usize = 10;

#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error("UnknownError")]
    UnknownError,
    #[error("Reqwest Error: {0}")]
    ReqwestError(String),
    #[error("Incorrect Sha256. File might be corrupted")]
    Sha256Error,
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
            .unwrap();

        let dirs = ProjectDirs::from("org", "beagleboard", "bb-imager").unwrap();

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

    pub async fn check_cache(self, _url: url::Url, sha256: [u8; 32]) -> Option<PathBuf> {
        let file_name = const_hex::encode(sha256);
        let file_path = self.dirs.cache_dir().join(file_name);

        if file_path.exists() {
            let x = sha256_file(&file_path).await.ok()?;
            if x == sha256 {
                return Some(file_path);
            }
        }

        None
    }

    pub async fn download(self, url: url::Url, sha256: [u8; 32]) -> Result<PathBuf> {
        let file_name = const_hex::encode(sha256);
        let file_path = self.dirs.cache_dir().join(file_name);

        if file_path.exists() {
            let x = sha256_file(&file_path).await?;
            if x == sha256 {
                return Ok(file_path);
            }

            // Delete old file
            let _ = tokio::fs::remove_file(&file_path).await;
        }

        let (mut file, tmp_file_path) = create_tmp_file(&file_path).await?;
        let response = self.client.get(url).send().await.map_err(Error::from)?;
        let response_stream = response.bytes_stream();
        let mut response_reader = StreamReader::new(
            response_stream.map(|x| x.map_err(|e| io::Error::new(io::ErrorKind::Other, e))),
        );

        tokio::io::copy_buf(&mut response_reader, &mut file).await?;

        let x = sha256_file(tmp_file_path.path()).await?;
        if x != sha256 {
            return Err(Error::Sha256Error.into());
        }

        tokio::fs::rename(tmp_file_path.path(), &file_path).await?;

        Ok(file_path)
    }

    pub async fn download_progress(
        &self,
        url: url::Url,
        sha256: [u8; 32],
        chan: &std::sync::mpsc::Sender<DownloadFlashingStatus>,
    ) -> Result<PathBuf> {
        let file_name = const_hex::encode(sha256);
        let file_path = self.dirs.cache_dir().join(file_name);

        if file_path.exists() {
            let _ = chan.send(DownloadFlashingStatus::VerifyingProgress(0.0));

            let hash = tokio::task::block_in_place(|| sha256_file_progress(&file_path, chan))?;
            if hash == sha256 {
                return Ok(file_path);
            }

            // Delete old file
            let _ = tokio::fs::remove_file(&file_path).await;
        }
        let _ = chan.send(DownloadFlashingStatus::DownloadingProgress(0.0));

        let (mut file, tmp_file_path) = create_tmp_file(&file_path).await?;
        let response = self.client.get(url).send().await.map_err(Error::from)?;

        let mut cur_pos = 0;
        let response_size = response.content_length();

        let mut response_stream = response.bytes_stream();

        let response_size = match response_size {
            Some(x) => x as usize,
            None => response_stream.size_hint().0,
        };

        while let Some(x) = response_stream.next().await {
            let mut data = x.map_err(Error::from)?;
            cur_pos += data.len();
            file.write_all_buf(&mut data).await?;

            let _ = chan.send(DownloadFlashingStatus::DownloadingProgress(
                (cur_pos as f32) / (response_size as f32),
            ));
        }

        let _ = chan.send(DownloadFlashingStatus::VerifyingProgress(0.0));

        let hash =
            tokio::task::block_in_place(|| sha256_file_progress(tmp_file_path.path(), chan))?;
        if hash != sha256 {
            return Err(Error::Sha256Error.into());
        }

        tokio::fs::rename(tmp_file_path.path(), &file_path).await?;

        Ok(file_path)
    }
}

async fn sha256_file(path: &Path) -> Result<[u8; 32]> {
    let file = tokio::fs::OpenOptions::new().read(true).open(path).await?;
    let mut reader = tokio::io::BufReader::new(file);

    let mut hasher = Sha256::new();
    let mut buffer = [0; BUF_SIZE];

    loop {
        let count = reader.read(&mut buffer).await?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    Ok(hasher
        .finalize()
        .as_slice()
        .try_into()
        .expect("SHA-256 is 32 bytes"))
}

async fn create_tmp_file(path: &Path) -> Result<(tokio::fs::File, TempFile)> {
    for i in 0..FILE_NAME_TRIES {
        let p = path.with_extension(format!("tmp.{}", i));
        if let Ok(f) = tokio::fs::File::create_new(&p).await {
            return Ok((f, TempFile::new(p)));
        }
    }

    Err(crate::error::Error::IoError(io::Error::new(
        io::ErrorKind::Other,
        "Failed to create tmp file",
    )))
}

#[derive(Clone)]
struct TempFile {
    path: PathBuf,
}

impl TempFile {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
