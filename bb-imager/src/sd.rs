//! Provide functionality to flash images to sd card

use crate::error::Result;
use crate::FlashingStatus;
use futures_util::Stream;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

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
    sd: crate::Destination,
    state: crate::State,
) -> impl Stream<Item = Result<crate::FlashingStatus>> {
    async_stream::try_stream! {
        yield FlashingStatus::Preparing;

        let size = img.size();

        let mut port = sd.open(&state).await?;
        tracing::info!("Opened File");
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

        if let Some(sha256) = img.sha256() {
            yield FlashingStatus::VerifyingProgress(0.0);

            port.seek(std::io::SeekFrom::Start(0)).await?;
            let verify_stream = crate::util::sha256_file_fixed_progress(port, size);

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

#[cfg(target_os = "linux")]
pub async fn destinations(
    state: &crate::State,
) -> Result<std::collections::HashSet<crate::Destination>> {
    use std::collections::HashSet;

    let block_devs = state
        .dbus_client
        .manager()
        .get_block_devices(Default::default())
        .await?
        .into_iter()
        .map(|x| state.dbus_client.object(x).unwrap())
        .collect::<Vec<_>>();

    let mut ans = HashSet::new();

    for obj in block_devs {
        if let Ok(block) = obj.block().await {
            if let Ok(drive) = state.dbus_client.drive_for_block(&block).await {
                if drive.removable().await.unwrap() && drive.media_removable().await.unwrap() {
                    let block = state
                        .dbus_client
                        .block_for_drive(&drive, true)
                        .await
                        .unwrap()
                        .into_inner()
                        .path()
                        .to_owned();
                    ans.insert(crate::Destination {
                        name: drive.id().await?,
                        size: Some(drive.size().await?),
                        block: Some(block.into()),
                    });
                }
            }
        }
    }

    Ok(ans)
}
