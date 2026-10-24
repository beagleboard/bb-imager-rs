use std::{io, sync::Arc};

use bb_flasher::img::{OsArchive, OsImage};
use bb_helper::file_stream::ReaderFileStream;
use tokio_util::task::AbortOnDropHandle;

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

#[derive(Debug, Clone)]
pub(crate) enum RemoteSource {
    File {
        path: Arc<std::path::Path>,
        size: u64,
    },
    Pipe {
        task: Arc<AbortOnDropHandle<io::Result<()>>>,
        rx: ReaderFileStream,
        size: u64,
    },
}

impl RemoteSource {
    pub(crate) fn from_img(
        img: &crate::db::OsImage,
        downloader: bb_downloader::Downloader,
        flasher: bb_config::config::Flasher,
    ) -> Self {
        let size = if matches!(flasher, bb_config::config::Flasher::SdCardBootfs) {
            img.image_download_size
        } else {
            img.extract_size
        };

        Self::new(
            *img.url.clone(),
            img.image_download_sha256,
            size as u64,
            downloader,
        )
    }

    pub(crate) fn new(
        url: url::Url,
        sha256: [u8; 32],
        size: u64,
        downloader: bb_downloader::Downloader,
    ) -> Self {
        let rt = tokio::runtime::Handle::current();

        let cache = downloader.check_cache_from_sha(sha256);
        if let Some(path) = cache {
            tracing::info!("Found the remote image in cache");
            return Self::File {
                path: path.into(),
                size,
            };
        }

        tracing::info!("Remote image not found in cache. Downloading");
        let (tx_stream, rx) = bb_helper::file_stream::file_stream().unwrap();

        let t: tokio::task::JoinHandle<io::Result<()>> = rt.spawn(async move {
            downloader
                .download_to_stream(url, sha256, tx_stream)
                .await
                .map_err(|e| {
                    let msg = format!("Error while downloading Os Image: {e}");
                    tracing::error!("{}", &msg);
                    io::Error::other(msg)
                })?;
            tracing::info!("Image download finished");
            Ok(())
        });

        Self::Pipe {
            task: Arc::new(AbortOnDropHandle::new(t)),
            rx,
            size,
        }
    }

    #[cfg(feature = "sd")]
    pub(crate) fn into_archive_fn(
        self,
        tx: Option<std::sync::mpsc::SyncSender<f32>>,
    ) -> impl FnOnce() -> io::Result<OsArchive> {
        move || match self {
            RemoteSource::File { path, .. } => OsArchive::from_path(&path, tx),
            RemoteSource::Pipe { task, rx, size } => OsArchive::from_piped(rx, task, size, tx),
        }
    }

    pub(crate) fn into_image_fn(self) -> impl FnOnce() -> io::Result<(OsImage, u64)> {
        move || match self {
            RemoteSource::File { path, size } => Ok((OsImage::from_path(&path)?, size)),
            RemoteSource::Pipe { task, rx, size } => {
                let img = OsImage::from_piped(rx, task, size)?;
                Ok((img, size))
            }
        }
    }
}
