use std::io;

use bb_flasher::img::{OsArchive, OsImage};
use bb_helper::file_stream::ReaderFileStream;
use tokio_util::task::AbortOnDropHandle;

#[derive(Debug, Clone)]
pub(crate) struct RemoteImage {
    name: Box<str>,
    url: Box<url::Url>,
    extract_sha256: [u8; 32],
    extract_size: u64,
    downloader: bb_downloader::Downloader,
}

impl RemoteImage {
    pub(crate) fn new(
        name: Box<str>,
        url: Box<url::Url>,
        extract_sha256: [u8; 32],
        extract_size: u64,
        downloader: bb_downloader::Downloader,
    ) -> Self {
        Self {
            name,
            url,
            extract_sha256,
            extract_size,
            downloader,
        }
    }

    pub(crate) fn file_name(&self) -> &str {
        self.url.path_segments().unwrap().next_back().unwrap()
    }

    fn open<C, P, R>(self, f_cache: C, f_pipe: P) -> impl FnOnce() -> io::Result<R>
    where
        C: FnOnce(&std::path::Path) -> io::Result<R>,
        P: FnOnce(ReaderFileStream, AbortOnDropHandle<io::Result<()>>, u64) -> io::Result<R>,
    {
        let rt = tokio::runtime::Handle::current();
        move || {
            let cache = self.downloader.check_cache_from_sha(self.extract_sha256);

            if let Some(path) = cache {
                tracing::info!("Found the remote image in cache");
                return f_cache(&path);
            }

            tracing::info!("Remote image not found in cache. Downloading");
            let (tx_stream, rx) = bb_helper::file_stream::file_stream()?;
            let sha = self.extract_sha256;

            let t: tokio::task::JoinHandle<io::Result<()>> = rt.spawn(async move {
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

            f_pipe(rx, AbortOnDropHandle::new(t), self.extract_size)
        }
    }

    #[cfg(feature = "sd")]
    pub(crate) fn into_archive_fn(
        self,
        tx: Option<std::sync::mpsc::SyncSender<f32>>,
    ) -> impl FnOnce() -> io::Result<OsArchive> {
        let tx_clone = tx.clone();
        self.open(
            move |p| OsArchive::from_path(p, tx_clone),
            move |rx, abort, es| OsArchive::from_piped(rx, abort, es, tx),
        )
    }

    pub(crate) fn into_image_fn(self) -> impl FnOnce() -> io::Result<(OsImage, u64)> {
        let extract_size = self.extract_size;
        self.open(
            move |p| Ok((OsImage::from_path(p)?, extract_size)),
            move |rx, abort, es| {
                let img = OsImage::from_piped(rx, abort, es)?;
                Ok((img, es))
            },
        )
    }
}

impl std::fmt::Display for RemoteImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[cfg(feature = "sd")]
#[derive(Debug, Clone)]
pub(crate) struct Bmap {
    pub(crate) url: Box<url::Url>,
    pub(crate) downloader: bb_downloader::Downloader,
}

#[cfg(feature = "sd")]
impl Bmap {
    pub(crate) fn into_fn(self) -> impl FnOnce() -> io::Result<Box<str>> {
        let rt = tokio::runtime::Handle::current();
        move || {
            let res = rt.block_on(async move { self.downloader.download(*self.url).await })?;
            std::fs::read_to_string(res).map(Into::into)
        }
    }
}
