//! Helper functions

use crate::{error::Result, BUF_SIZE};
use std::{io::Read, path::Path};

use sha2::{Digest, Sha256};

pub(crate) fn sha256_file_progress(
    path: &Path,
    chan: &std::sync::mpsc::Sender<crate::DownloadFlashingStatus>,
) -> Result<[u8; 32]> {
    let mut file = std::fs::File::open(path)?;
    let file_len = file.metadata()?.len() as f32;

    let mut hasher = Sha256::new();
    let mut buffer = [0; BUF_SIZE];
    let mut pos = 0;

    loop {
        let count = file.read(&mut buffer)?;
        pos += count;

        let _ = chan.send(crate::DownloadFlashingStatus::VerifyingProgress(
            pos as f32 / file_len,
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

pub(crate) fn sha256_file_fixed_progress(
    file: std::fs::File,
    size: u64,
    chan: &std::sync::mpsc::Sender<crate::DownloadFlashingStatus>,
) -> Result<[u8; 32]> {
    let mut reader = std::io::BufReader::new(file);

    let mut hasher = Sha256::new();
    let mut buffer = [0; BUF_SIZE];
    let mut pos = 0;

    while pos < size as usize {
        let count = reader.read(&mut buffer)?;
        let count = std::cmp::min(size as usize - pos, count);

        pos += count;
        let _ = chan.send(crate::DownloadFlashingStatus::VerifyingProgress(
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
