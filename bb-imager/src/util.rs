//! Helper functions

use crate::{error::Result, BUF_SIZE};
use std::{io::Read, path::Path};

use sha2::{Digest, Sha256};

pub(crate) fn sha256_file_progress(
    path: &Path,
    chan: &std::sync::mpsc::Sender<crate::DownloadFlashingStatus>,
) -> Result<[u8; 32]> {
    let file = std::fs::File::open(path)?;
    let file_len = file.metadata()?.len();

    sha256_reader_progress(file, file_len, chan)
}

pub(crate) fn sha256_reader_progress<R: Read>(
    mut reader: R,
    size: u64,
    chan: &std::sync::mpsc::Sender<crate::DownloadFlashingStatus>,
) -> Result<[u8; 32]> {
    let mut hasher = Sha256::new();
    let mut buffer = [0; BUF_SIZE];
    let mut pos = 0;

    loop {
        let count = reader.read(&mut buffer)?;
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
