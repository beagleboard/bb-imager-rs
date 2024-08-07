//! Helper functions

use crate::{error::Result, BUF_SIZE};
use std::path::Path;

use sha2::{Digest, Sha256};

pub(crate) async fn sha256_file_progress(
    path: &Path,
    chan: &tokio::sync::mpsc::Sender<crate::DownloadFlashingStatus>,
) -> Result<[u8; 32]> {
    let file = tokio::fs::File::open(path).await?;
    let file_len = file.metadata().await?.len();

    sha256_reader_progress(file, file_len, chan).await
}

pub(crate) async fn sha256_reader_progress<R: tokio::io::AsyncReadExt + Unpin>(
    mut reader: R,
    size: u64,
    chan: &tokio::sync::mpsc::Sender<crate::DownloadFlashingStatus>,
) -> Result<[u8; 32]> {
    let mut hasher = Sha256::new();
    let mut buffer = [0; BUF_SIZE];
    let mut pos = 0;

    loop {
        let count = reader.read(&mut buffer).await?;
        pos += count;

        let _ = chan.try_send(crate::DownloadFlashingStatus::VerifyingProgress(
            pos as f32 / size as f32,
        ));

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
