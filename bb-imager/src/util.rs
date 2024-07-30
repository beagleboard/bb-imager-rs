//! Helper functions

use crate::{error::Result, BUF_SIZE};
use std::path::Path;

use futures::SinkExt;
use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt;

pub(crate) async fn sha256_file_progress(
    path: &Path,
    chan: &mut futures::channel::mpsc::UnboundedSender<crate::DownloadFlashingStatus>,
) -> Result<[u8; 32]> {
    let file = tokio::fs::File::open(path).await?;
    let file_len = file.metadata().await?.len() as f32;
    let mut reader = tokio::io::BufReader::new(file);

    let mut hasher = Sha256::new();
    let mut buffer = [0; BUF_SIZE];
    let mut pos = 0;

    loop {
        let count = reader.read(&mut buffer).await?;
        pos += count;

        let _ = chan
            .send(crate::DownloadFlashingStatus::VerifyingProgress(
                pos as f32 / file_len,
            ))
            .await;

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

pub(crate) async fn sha256_file_fixed_progress(
    file: tokio::fs::File,
    size: u64,
    chan: &mut futures::channel::mpsc::UnboundedSender<crate::DownloadFlashingStatus>,
) -> Result<[u8; 32]> {
    let mut reader = tokio::io::BufReader::new(file);

    let mut hasher = Sha256::new();
    let mut buffer = [0; BUF_SIZE];
    let mut pos = 0;

    while pos < size as usize {
        let count = reader.read(&mut buffer).await?;
        let count = std::cmp::min(size as usize - pos, count);

        pos += count;
        let _ = chan
            .send(crate::DownloadFlashingStatus::VerifyingProgress(
                pos as f32 / size as f32,
            ))
            .await;

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
