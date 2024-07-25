//! Module for downloading remote images for flashing

use futures_core::Stream;
use std::time::Duration;

pub fn download(
    remote_zip: url::Url,
    zip_sha256: Vec<u8>,
    path_in_zip: String,
    img_sha256: Vec<u8>,
) -> impl Stream<Item = Result<crate::DownloadStatus, String>> {
    async_stream::try_stream! {
        yield crate::DownloadStatus::Downloading;

        let res = tokio::task::spawn_blocking(move || {
            let img = &data_downloader::InZipDownloadRequest {
                parent: &data_downloader::DownloadRequest {
                    url: &remote_zip.to_string(),
                    sha256_hash: &zip_sha256,
                },
                path: &path_in_zip,
                sha256_hash: &img_sha256
            };

            data_downloader::DownloaderBuilder::new()
                .retry_attempts(0)
                .timeout(Some(Duration::from_secs(10)))
                .build()
                .unwrap()
                .get_path(img)
        }).await.unwrap().map_err(|e| e.to_string())?;

        yield crate::DownloadStatus::Finished(res);
    }
}
