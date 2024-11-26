//! Helper functions

use crate::error::Result;
use std::path::Path;

use sha2::{Digest, Sha256};

const BUF_SIZE: usize = 4 * 1024;

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

        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);

        pos += count;
        let _ = chan.try_send(crate::DownloadFlashingStatus::VerifyingProgress(
            pos as f32 / size as f32,
        ));
    }

    let hash = hasher
        .finalize()
        .as_slice()
        .try_into()
        .expect("SHA-256 is 32 bytes");

    Ok(hash)
}

/// TODO: Remove this once https://gitlab.com/robert.ernst.paf/bin_file/-/merge_requests/2 is
/// merged
pub(crate) fn bin_file_from_str<S>(contents: S) -> Result<bin_file::BinFile, bin_file::Error>
where
    S: AsRef<str>,
{
    let mut binfile = bin_file::BinFile::new();
    let lines: Vec<&str> = contents.as_ref().lines().collect();
    binfile.add_strings(lines, false)?;
    Ok(binfile)
}
