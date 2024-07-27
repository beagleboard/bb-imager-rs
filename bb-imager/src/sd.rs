//! Provide functionality to flash images to sd card

use crate::error::Result;
use crate::FlashingStatus;
use futures_util::Stream;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const BUF_SIZE: usize = 8 * 1024;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Sha256 verification error")]
    Sha256VerificationError,
    #[error("Failed to get removable flash drives")]
    DriveFetchError,
}

pub fn flash(
    mut img: crate::img::OsImage,
    port_path: String,
) -> impl Stream<Item = Result<crate::FlashingStatus>> {
    async_stream::try_stream! {
        yield FlashingStatus::Preparing;

        let size = img.size();

        let port = tokio::fs::OpenOptions::new().read(true).write(true).open(&port_path).await?;
        let mut port = tokio::io::BufWriter::new(port);
        let mut buf = [0u8; BUF_SIZE];
        let mut pos = 0;

        yield FlashingStatus::FlashingProgress(0.0);

        loop {
            let count = img.read(&mut buf).await?;
            pos += count;

            yield FlashingStatus::FlashingProgress(pos as f32 / size as f32);

            if count == 0 {
                break;
            }

            port.write_all(&buf[..count]).await?;
        }

        drop(port);

        if let Some(sha256) = img.sha256() {
            yield FlashingStatus::VerifyingProgress(0.0);

            let verify_stream = crate::util::sha256_file_progress(port_path);

            for await v in verify_stream {
                match v? {
                    crate::util::Sha256State::Progress(x) => yield FlashingStatus::VerifyingProgress(x),
                    crate::util::Sha256State::Finish(x) => {
                        if x != sha256 {
                            Err(Error::Sha256VerificationError)?;
                            return;
                        }
                    },
                }
            }
        }

        yield FlashingStatus::Finished
    }
}

// pub fn format(dev: &Path) -> io::Result<()> {
//     let disk = std::fs::OpenOptions::new()
//         .read(true)
//         .write(true)
//         .open(dev)?;
//     fatfs::format_volume(disk, fatfs::FormatVolumeOptions::new())
// }

pub fn destinations() -> Result<Vec<String>> {
    let ans = drives::get_devices()
        .unwrap()
        .into_iter()
        .filter(|x| x.is_removable)
        .map(|x| format!("/dev/{}", x.name))
        .collect::<Vec<_>>();

    Ok(ans)
}
