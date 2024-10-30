//! Provide functionality to flash images to sd card

use std::io::Read;

use crate::DownloadFlashingStatus;
use crate::{error::Result, BUF_SIZE};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Sha256 verification error")]
    Sha256VerificationError,
    #[error("Failed to get removable flash drives")]
    DriveFetchError,
}

fn read_aligned(img: &mut crate::img::OsImage, buf: &mut [u8]) -> Result<usize> {
    let mut pos = 0;

    loop {
        let count = img.read(&mut buf[pos..])?;

        if count == 0 {
            break;
        }

        pos += count;

        if pos % 512 == 0 {
            break;
        }
    }

    if pos == 0 || pos % 512 == 0 {
        Ok(pos)
    } else {
        let rem = pos % 512;
        buf[pos..rem].fill(0);
        Ok(pos + rem)
    }
}

pub(crate) async fn flash<W: AsyncReadExt + AsyncWriteExt + AsyncSeekExt + Unpin>(
    mut img: crate::img::OsImage,
    mut sd: W,
    chan: &tokio::sync::mpsc::Sender<DownloadFlashingStatus>,
    verify: bool,
) -> Result<()> {
    let size = img.size();

    let mut buf = [0u8; BUF_SIZE];
    let mut pos = 0;

    let _ = chan.try_send(DownloadFlashingStatus::FlashingProgress(0.0));

    loop {
        let count = read_aligned(&mut img, &mut buf)?;
        if count == 0 {
            break;
        }

        pos += count;

        let _ = chan.try_send(DownloadFlashingStatus::FlashingProgress(
            pos as f32 / size as f32,
        ));

        sd.write_all(&buf[..count]).await?;
    }

    if verify {
        let sha256 = img.sha256();
        let _ = chan.try_send(DownloadFlashingStatus::VerifyingProgress(0.0));

        sd.seek(std::io::SeekFrom::Start(0)).await?;
        let hash = crate::util::sha256_reader_progress(sd.take(size), size, chan).await?;

        if hash != sha256 {
            return Err(Error::Sha256VerificationError.into());
        }
    }

    Ok(())
}

// pub fn format(dev: &Path) -> io::Result<()> {
//     let disk = std::fs::OpenOptions::new()
//         .read(true)
//         .write(true)
//         .open(dev)?;
//     fatfs::format_volume(disk, fatfs::FormatVolumeOptions::new())
// }

#[cfg(not(target_os = "macos"))]
pub fn destinations() -> std::collections::HashSet<crate::Destination> {
    rs_drivelist::drive_list()
        .unwrap()
        .into_iter()
        .filter(|x| x.isRemovable)
        .filter(|x| !x.isVirtual)
        .map(|x| crate::Destination::sd_card(x.description, x.size, x.raw))
        .collect()
}

#[cfg(target_os = "macos")]
pub fn destinations() -> std::collections::HashSet<crate::Destination> {
    crate::pal::macos::rs_drivelist::diskutil()
        .unwrap()
        .into_iter()
        .filter(|x| x.isRemovable)
        .filter(|x| !x.isVirtual)
        .map(|x| crate::Destination::sd_card(x.description, x.size, x.raw))
        .collect()
}
