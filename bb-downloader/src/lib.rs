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
//!
//! # Sample Usage
//!
//! ```no_run
//! #[tokio::main]
//! async fn main() {
//!     let downloader = bb_downloader::Downloader::new("/tmp").unwrap();
//!
//!     let sha = [0u8; 32];
//!     let url = "https://example.com/img.jpg";
//!
//!     // Download with just URL
//!     downloader.download(url, None).await.unwrap();
//!
//!     // Check if the file is in cache
//!     assert!(downloader.check_cache_from_url(url).is_some());
//!
//!     // Will fetch directly from cache instead of re-downloading
//!     downloader.download(url, None).await.unwrap();
//!
//!     // Will fetch directly from cache instead of re-downloading
//!     assert!(!downloader.check_cache_from_sha(sha).await.is_some());
//!
//!     // Will re-download the file
//!     downloader.download_with_sha(url, sha, None).await.unwrap();
//!
//!     assert!(downloader.check_cache_from_sha(sha).await.is_some());
//! }
//! ```

use futures::{Stream, StreamExt, channel::mpsc};
#[cfg(feature = "json")]
use serde::de::DeserializeOwned;
use sha2::{Digest as _, Sha256};
use std::{
    io,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

pub use reqwest::IntoUrl;

/// Simple downloader that caches files in the provided directory. Uses SHA256 to determine if the
/// file is already downloaded.
///
/// Either SHA256 or URL can be used for caching files. However, both are not interchangable. If
/// SHA256 cannot be used to check files that were downloaded with just URL, and vice versa.
///
/// # Invalidate Cache
///
/// Using SHA256 should be prefered when it is known in advance since it allows performing SHA256
/// verficiation on the downloaded file. Additionally, it also adds capability to invalidate cached
/// file.
///
/// Files downloaded with just URL cannot be invalidated without changing the URL, or deleting the
/// file manually.
///
/// # Thread Safety
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
    pub fn new<P: Into<PathBuf>>(cache_dir: P) -> io::Result<Self> {
        let cache_dir = cache_dir.into();

        if !cache_dir.exists() {
            let _ = std::fs::create_dir_all(&cache_dir);
        }

        if cache_dir.exists() && !cache_dir.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                "cache_dir should be a directory",
            ));
        }

        let client = reqwest::Client::builder()
            .user_agent(env!("CARGO_PKG_NAME"))
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(15))
            .build()
            .expect("Unsupported OS");

        Ok(Self { client, cache_dir })
    }

    /// Check if a downloaded file with a particular SHA256 is already in cache.
    pub async fn check_cache_from_sha(&self, sha256: [u8; 32]) -> Option<PathBuf> {
        let file_path = self.path_from_sha(sha256);

        if file_path.exists() {
            if let Ok(hash) = sha256_from_path(&file_path).await
                && hash == sha256
            {
                return Some(file_path);
            }

            // Delete old file
            let _ = tokio::fs::remove_file(&file_path).await;
        }

        None
    }

    /// Check if a downloaded file is already in cache.
    ///
    /// [`check_cache_from_sha`](Self::check_cache_from_sha) should be prefered in cases when SHA256
    /// of the file to download is already known.
    pub fn check_cache_from_url<U: reqwest::IntoUrl>(&self, url: U) -> Option<PathBuf> {
        // Use hash of url for file name
        let file_path = self.path_from_url(&url.into_url().ok()?);
        if file_path.exists() {
            Some(file_path)
        } else {
            None
        }
    }

    /// Download a JSON file without caching the contents. Should be used when there is no point in
    /// caching the file.
    #[cfg(feature = "json")]
    pub async fn download_json_no_cache<T, U>(&self, url: U) -> io::Result<T>
    where
        T: DeserializeOwned,
        U: reqwest::IntoUrl,
    {
        self.client
            .get(url)
            .send()
            .await
            .map_err(io::Error::other)?
            .json()
            .await
            .map_err(io::Error::other)
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
    pub async fn download<U: reqwest::IntoUrl>(
        &self,
        url: U,
        chan: Option<mpsc::Sender<f32>>,
    ) -> io::Result<PathBuf> {
        let url = url.into_url().map_err(io::Error::other)?;

        // Check cache
        if let Some(p) = self.check_cache_from_url(url.clone()) {
            return Ok(p);
        }

        self.download_no_cache(url, chan).await
    }

    /// Downloads the file without checking cache.
    ///
    /// [`download_with_sha`](Self::download_with_sha) should be prefered when the SHA256 of the
    /// file is known in advance.
    ///
    /// # Progress
    ///
    /// Download progress can be optionally tracked using a [`futures::channel::mpsc`].
    ///
    /// # Differences from [Self::download]
    ///
    /// This function does not check if the file is present in cache, and will ovewrite the old
    /// cached file. The file is still cached in the end.
    pub async fn download_no_cache<U: reqwest::IntoUrl>(
        &self,
        url: U,
        mut chan: Option<mpsc::Sender<f32>>,
    ) -> io::Result<PathBuf> {
        let url = url.into_url().map_err(io::Error::other)?;

        let file_path = self.path_from_url(&url);
        chan_send(chan.as_mut(), 0.0);

        let mut cur_pos = 0;
        let mut file = AsyncTempFile::new()?;
        {
            let mut file = tokio::io::BufWriter::new(&mut file.0);

            let response = self
                .client
                .get(url)
                .send()
                .await
                .map_err(io::Error::other)?;
            let response_size = response.content_length();
            let mut response_stream = response.bytes_stream();

            let response_size = match response_size {
                Some(x) => x as usize,
                None => response_stream.size_hint().0,
            };

            while let Some(x) = response_stream.next().await {
                let mut data = x.map_err(io::Error::other)?;
                cur_pos += data.len();
                file.write_all_buf(&mut data).await?;
                chan_send(chan.as_mut(), (cur_pos as f32) / (response_size as f32));
            }

            file.flush().await?
        }

        file.persist(&file_path).await?;
        Ok(file_path)
    }

    /// Downloads the file and streams the content to pipe. This allows not having to wait for the
    /// download to finish to use the partial file.
    ///
    /// Uses SHA256 to verify that the file in cache is valid.
    pub async fn download_to_stream<U: reqwest::IntoUrl>(
        self,
        url: U,
        sha256: [u8; 32],
        mut writer: bb_helper::file_stream::WriterFileStream,
    ) -> io::Result<()> {
        let url = url.into_url().map_err(io::Error::other)?;
        tracing::debug!(
            "Download {:?} with sha256: {:?}",
            url,
            const_hex::encode(sha256)
        );

        let file_path = self.path_from_sha(sha256);

        {
            let mut file = tokio::io::BufWriter::new(&mut writer);

            let response = self
                .client
                .get(url)
                .send()
                .await
                .map_err(io::Error::other)?;

            let mut response_stream = response.bytes_stream();

            let mut hasher = Sha256::new();

            while let Some(x) = response_stream.next().await {
                tracing::debug!("Got buf");
                let mut data = x.map_err(io::Error::other)?;
                hasher.update(&data);
                file.write_all_buf(&mut data).await?;
            }

            let hash: [u8; 32] = hasher
                .finalize()
                .as_slice()
                .try_into()
                .expect("SHA-256 is 32 bytes");

            if hash != sha256 {
                tracing::error!(
                    "Expected SHA256: {}, got {}",
                    const_hex::encode(sha256),
                    const_hex::encode(hash)
                );
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Invalid SHA256",
                ));
            }
            file.flush().await?;
        }

        tracing::info!("Saving donwloaded file to disk");
        writer.persist(&file_path).await
    }

    /// Checks if the file is present in cache. If the file is present, returns path to it. Else
    /// downloads the file.
    ///
    /// Uses SHA256 to verify that the file in cache is valid.
    ///
    /// # Progress
    ///
    /// Download progress can be optionally tracked using a [`futures::channel::mpsc`].
    pub async fn download_with_sha<U: reqwest::IntoUrl>(
        &self,
        url: U,
        sha256: [u8; 32],
        mut chan: Option<mpsc::Sender<f32>>,
    ) -> io::Result<PathBuf> {
        let url = url.into_url().map_err(io::Error::other)?;
        tracing::debug!(
            "Download {:?} with sha256: {:?}",
            url,
            const_hex::encode(sha256)
        );

        if let Some(p) = self.check_cache_from_sha(sha256).await {
            return Ok(p);
        }

        let file_path = self.path_from_sha(sha256);
        chan_send(chan.as_mut(), 0.0);

        let mut file = AsyncTempFile::new()?;
        {
            let mut file = tokio::io::BufWriter::new(&mut file.0);

            let response = self
                .client
                .get(url)
                .send()
                .await
                .map_err(io::Error::other)?;

            let mut cur_pos = 0;
            let response_size = response.content_length();

            let mut response_stream = response.bytes_stream();

            let response_size = match response_size {
                Some(x) => x as usize,
                None => response_stream.size_hint().0,
            };

            let mut hasher = Sha256::new();

            while let Some(x) = response_stream.next().await {
                let mut data = x.map_err(io::Error::other)?;
                cur_pos += data.len();
                hasher.update(&data);
                file.write_all_buf(&mut data).await?;

                chan_send(chan.as_mut(), (cur_pos as f32) / (response_size as f32));
            }

            let hash: [u8; 32] = hasher
                .finalize()
                .as_slice()
                .try_into()
                .expect("SHA-256 is 32 bytes");

            if hash != sha256 {
                tracing::error!(
                    "Expected SHA256: {}, got {}",
                    const_hex::encode(sha256),
                    const_hex::encode(hash)
                );
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Invalid SHA256",
                ));
            }
            file.flush().await?;
        }

        file.persist(&file_path).await?;
        Ok(file_path)
    }

    fn path_from_url(&self, url: &reqwest::Url) -> PathBuf {
        let fext = Path::new(url.path()).extension().expect("Invalid URL");
        let file_name: [u8; 32] = Sha256::new()
            .chain_update(url.as_str())
            .finalize()
            .as_slice()
            .try_into()
            .expect("SHA-256 is 32 bytes");
        self.path_from_sha(file_name).with_extension(fext)
    }

    fn path_from_sha(&self, sha256: [u8; 32]) -> PathBuf {
        let file_name = const_hex::encode(sha256);
        self.cache_dir.join(file_name)
    }
}

async fn sha256_from_path(p: &Path) -> io::Result<[u8; 32]> {
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

struct AsyncTempFile(tokio::fs::File);

impl AsyncTempFile {
    fn new() -> io::Result<Self> {
        let f = tempfile::tempfile()?;
        Ok(Self(tokio::fs::File::from_std(f)))
    }

    async fn persist(&mut self, path: &Path) -> io::Result<()> {
        let mut f = tokio::fs::File::create(path).await?;
        self.0.seek(io::SeekFrom::Start(0)).await?;

        tokio::io::copy(&mut self.0, &mut f).await?;

        // Causes errors if not present
        f.flush().await?;

        Ok(())
    }
}

impl From<std::fs::File> for AsyncTempFile {
    fn from(value: std::fs::File) -> Self {
        Self(tokio::fs::File::from_std(value))
    }
}
