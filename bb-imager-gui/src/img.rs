use std::io;
use std::path::Path;
use std::sync::Arc;
#[cfg(feature = "sd")]
use std::sync::Mutex;

use bb_flasher::img::{OsArchive, OsImage};
use bb_helper::file_stream::ReaderFileStream;
use tokio_util::task::AbortOnDropHandle;

#[derive(Debug, Clone)]
pub(crate) struct RemoteImage {
    item: RemoteItem,
}

impl RemoteImage {
    pub(crate) fn new(
        img: &crate::db::OsImage,
        downloader: bb_downloader::Downloader,
        flasher: bb_config::config::Flasher,
    ) -> Self {
        let size = if matches!(flasher, bb_config::config::Flasher::SdCardBootfs) {
            img.image_download_size
        } else {
            img.extract_size
        };

        Self {
            item: RemoteItem::new(
                img.url.clone(),
                img.image_download_sha256,
                size as u64,
                downloader,
            ),
        }
    }

    /// See [`RemoteItem::open`].
    pub(crate) fn open(self, n: usize) -> io::Result<Vec<RemoteSource>> {
        self.item.open(n)
    }
}

#[cfg(feature = "sd")]
#[derive(Debug, Clone)]
pub(crate) struct Bmap {
    url: Box<url::Url>,
    downloader: bb_downloader::Downloader,
    // Shared by every destination of a flash, so the bmap is only fetched once.
    cache: Arc<Mutex<Option<Box<str>>>>,
}

#[cfg(feature = "sd")]
impl Bmap {
    pub(crate) fn new(url: Box<url::Url>, downloader: bb_downloader::Downloader) -> Self {
        Self {
            url,
            downloader,
            cache: Default::default(),
        }
    }

    pub(crate) fn into_fn(self) -> impl FnOnce() -> io::Result<Box<str>> {
        let rt = tokio::runtime::Handle::current();
        move || {
            let mut cache = self.cache.lock().unwrap();
            if let Some(x) = cache.as_ref() {
                return Ok(x.clone());
            }

            let res = rt.block_on(async move { self.downloader.download(*self.url).await })?;
            let bmap: Box<str> = std::fs::read_to_string(res)?.into();
            *cache = Some(bmap.clone());
            Ok(bmap)
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteItem {
    url: Box<url::Url>,
    sha256: [u8; 32],
    // Should be compressed size for image, uncompressed size for archive.
    size: u64,
    downloader: bb_downloader::Downloader,
}

impl RemoteItem {
    pub(crate) const fn new(
        url: Box<url::Url>,
        sha256: [u8; 32],
        size: u64,
        downloader: bb_downloader::Downloader,
    ) -> Self {
        Self {
            url,
            sha256,
            size,
            downloader,
        }
    }

    /// Open `n` independent sources for the item, one per destination.
    ///
    /// If the item is not in cache, a single download is started and every source streams from
    /// it. The download is aborted once all sources are dropped.
    pub(crate) fn open(self, n: usize) -> io::Result<Vec<RemoteSource>> {
        let size = self.size;

        if let Some(path) = self.downloader.check_cache_from_sha(self.sha256) {
            tracing::info!("Found the remote image in cache");
            let path: Arc<Path> = path.into();
            return Ok((0..n)
                .map(|_| RemoteSource::Cached(path.clone(), size))
                .collect());
        }

        tracing::info!("Remote image not found in cache. Downloading");
        let (tx_stream, rx) = bb_helper::file_stream::file_stream()?;

        let sha = self.sha256;
        let t: tokio::task::JoinHandle<io::Result<()>> =
            tokio::runtime::Handle::current().spawn(async move {
                self.downloader
                    .download_to_stream(*self.url, sha, tx_stream)
                    .await
                    .map_err(|e| {
                        let msg = format!("Error while downloading Os Image: {e}");
                        tracing::error!("{}", &msg);
                        io::Error::other(msg)
                    })?;
                tracing::info!("Image download finished");
                Ok(())
            });
        let background = Arc::new(AbortOnDropHandle::new(t));

        // Each clone reads the whole stream independently.
        let mut srcs: Vec<_> = (1..n)
            .map(|_| RemoteSource::Piped(rx.clone(), background.clone(), size))
            .collect();
        srcs.push(RemoteSource::Piped(rx, background, size));
        Ok(srcs)
    }
}

/// One reader of a [`RemoteItem`], from cache or from an in-progress download.
pub(crate) enum RemoteSource {
    Cached(Arc<Path>, u64),
    Piped(
        ReaderFileStream,
        Arc<AbortOnDropHandle<io::Result<()>>>,
        u64,
    ),
}

impl std::fmt::Debug for RemoteSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cached(p, _) => f.debug_tuple("Cached").field(p).finish(),
            Self::Piped(..) => f.write_str("Piped"),
        }
    }
}

impl RemoteSource {
    #[cfg(feature = "sd")]
    pub(crate) fn into_archive_fn(
        self,
        tx: Option<std::sync::mpsc::SyncSender<f32>>,
    ) -> impl FnOnce() -> io::Result<OsArchive> {
        move || match self {
            Self::Cached(p, _) => OsArchive::from_path(&p, tx),
            Self::Piped(rx, abort, size) => OsArchive::from_piped(rx, abort, size, tx),
        }
    }

    pub(crate) fn into_image_fn(self) -> impl FnOnce() -> io::Result<(OsImage, u64)> {
        move || match self {
            Self::Cached(p, size) => Ok((OsImage::from_path(&p)?, size)),
            Self::Piped(rx, abort, size) => Ok((OsImage::from_piped(rx, abort, size)?, size)),
        }
    }
}
