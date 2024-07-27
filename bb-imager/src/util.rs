//! Helper functions

use crate::error::Result;
use std::path::PathBuf;

use futures_util::Stream;
use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt;

const BUF_SIZE: usize = 8 * 1024;

pub(crate) enum Sha256State {
    Progress(f32),
    Finish([u8; 32]),
}

pub(crate) fn sha256_file_progress(
    path: impl Into<PathBuf>,
) -> impl Stream<Item = Result<Sha256State>> {
    async_stream::try_stream! {
        let file = tokio::fs::File::open(path.into()).await?;
        let file_len = file.metadata().await?.len() as f32;
        let mut reader = tokio::io::BufReader::new(file);

        let mut hasher = Sha256::new();
        let mut buffer = [0; BUF_SIZE];
        let mut pos = 0;

        loop {
            let count = reader.read(&mut buffer).await?;
            pos += count;
            yield Sha256State::Progress(pos as f32 / file_len);
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

        yield Sha256State::Finish(hash);
    }
}

pub(crate) fn sha256_file_fixed_progress(
    file: tokio::fs::File,
    size: u64,
) -> impl Stream<Item = Result<Sha256State>> {
    async_stream::try_stream! {
        let mut reader = tokio::io::BufReader::new(file);

        let mut hasher = Sha256::new();
        let mut buffer = [0; BUF_SIZE];
        let mut pos = 0;

        while pos < size as usize {
            let count = reader.read(&mut buffer).await?;
            let count = std::cmp::min(size as usize - pos, count);

            pos += count;
            yield Sha256State::Progress(pos as f32 / size as f32);

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

        yield Sha256State::Finish(hash);
    }
}
