//! Provide functionality to flash images to sd card

use std::io::{Read, Seek, SeekFrom, Write};

use crate::error::Result;
use crate::DownloadFlashingStatus;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

const BUF_SIZE: usize = 128 * 1024;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Sha256 verification error")]
    Sha256Verification,
    #[error("Failed to get removable flash drives")]
    DriveFetch,
    #[error("Failed to format drive {0}")]
    Format(String),
    #[error("Failed to customize flashed image {0}")]
    Customization(String),
}

fn read_aligned(img: &mut crate::img::OsImage, buf: &mut [u8]) -> Result<usize> {
    let mut pos = 0;

    loop {
        let count = img.read(&mut buf[pos..])?;

        if count == 0 {
            let end = pos + pos % 512;
            buf[pos..end].fill(0);
            return Ok(end);
        }

        pos += count;
        // The buffer size is always a multiple of 512
        if pos % 512 == 0 {
            return Ok(pos);
        }
    }
}

pub(crate) async fn flash<W>(
    mut img: crate::img::OsImage,
    mut sd: W,
    chan: &tokio::sync::mpsc::Sender<DownloadFlashingStatus>,
    verify: bool,
) -> Result<()>
where
    W: AsyncReadExt + AsyncWriteExt + AsyncSeekExt + Unpin,
{
    let size = img.size();

    let mut buf = [0u8; BUF_SIZE];
    let mut pos = 0;

    let _ = chan.try_send(DownloadFlashingStatus::FlashingProgress(0.0));

    loop {
        let count = read_aligned(&mut img, &mut buf)?;
        if count == 0 {
            break;
        }

        sd.write_all(&buf[..count]).await?;

        pos += count;
        let _ = chan.try_send(DownloadFlashingStatus::FlashingProgress(
            pos as f32 / size as f32,
        ));
    }

    if verify {
        let sha256 = img.sha256();
        let _ = chan.try_send(DownloadFlashingStatus::VerifyingProgress(0.0));

        sd.seek(std::io::SeekFrom::Start(0)).await?;
        let hash = crate::util::sha256_reader_progress(sd.take(size), size, chan).await?;

        if hash != sha256 {
            tracing::debug!("Image SHA256: {}", const_hex::encode(sha256));
            tracing::debug!("Sd SHA256: {}", const_hex::encode(hash));
            return Err(Error::Sha256Verification.into());
        }
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn destinations() -> std::collections::HashSet<crate::Destination> {
    rs_drivelist::drive_list()
        .expect("Unsupported OS for Sd Card")
        .into_iter()
        .filter(|x| x.isRemovable)
        .filter(|x| !x.isVirtual)
        .map(|x| crate::Destination::sd_card(x.description, x.size, x.raw))
        .collect()
}

#[cfg(target_os = "macos")]
pub fn destinations() -> std::collections::HashSet<crate::Destination> {
    crate::pal::macos::rs_drivelist::diskutil()
        .expect("Unsupported OS for Sd Card")
        .into_iter()
        .filter(|x| x.isRemovable)
        .filter(|x| !x.isVirtual)
        .map(|x| crate::Destination::sd_card(x.description, x.size, x.raw))
        .collect()
}

#[derive(Clone, Debug)]
pub struct FlashingSdLinuxConfig {
    pub verify: bool,
    pub hostname: Option<String>,
    pub timezone: Option<String>,
    pub keymap: Option<String>,
    pub user: Option<(String, String)>,
    pub wifi: Option<(String, String)>,
}

impl FlashingSdLinuxConfig {
    pub fn customize<D: std::io::Write + std::io::Seek + std::io::Read>(
        &self,
        dst: &mut D,
    ) -> crate::error::Result<()> {
        let boot_partition = {
            let mbr = mbrman::MBR::read_from(dst, 512)
                .map_err(|e| Error::Customization(format!("Failed to read mbr: {e}")))?;

            let boot_part = mbr.get(1).ok_or(Error::Customization(
                "Failed to get boot partition".to_string(),
            ))?;
            assert_eq!(boot_part.sys, 12);
            let start_offset: u64 = (boot_part.starting_lba * mbr.sector_size).into();
            let end_offset: u64 =
                start_offset + u64::from(boot_part.sectors) * u64::from(mbr.sector_size);
            let slice = fscommon::StreamSlice::new(dst, start_offset, end_offset)
                .map_err(|_| Error::Customization("Failed to read partition".to_string()))?;
            let boot_stream = fscommon::BufStream::new(slice);
            fatfs::FileSystem::new(boot_stream, fatfs::FsOptions::new())
                .map_err(|e| Error::Customization(format!("Failed to open boot partition: {e}")))?
        };

        let boot_root = boot_partition.root_dir();

        if self.hostname.is_some()
            || self.timezone.is_some()
            || self.keymap.is_some()
            || self.user.is_some()
            || self.wifi.is_some()
        {
            let mut sysconf = boot_root
                .create_file("sysconf.txt")
                .map_err(|e| Error::Customization(format!("Failed to create sysconf.txt: {e}")))?;
            sysconf.seek(SeekFrom::End(0)).map_err(|e| {
                Error::Customization(format!("Failed to seek to end of sysconf.txt: {e}"))
            })?;

            if let Some(h) = &self.hostname {
                sysconf
                    .write_all(format!("hostname={h}\n").as_bytes())
                    .map_err(|e| {
                        Error::Customization(format!(
                            "Failed to write hostname to sysconf.txt: {e}"
                        ))
                    })?;
            }

            if let Some(tz) = &self.timezone {
                sysconf
                    .write_all(format!("timezone={tz}\n").as_bytes())
                    .map_err(|e| {
                        Error::Customization(format!(
                            "Failed to write timezone to sysconf.txt: {e}"
                        ))
                    })?;
            }

            if let Some(k) = &self.keymap {
                sysconf
                    .write_all(format!("keymap={k}\n").as_bytes())
                    .map_err(|e| {
                        Error::Customization(format!("Failed to write keymap to sysconf.txt: {e}"))
                    })?;
            }

            if let Some((u, p)) = &self.user {
                sysconf
                    .write_all(format!("user_name={u}\n").as_bytes())
                    .map_err(|e| {
                        Error::Customization(format!(
                            "Failed to write user_name to sysconf.txt: {e}"
                        ))
                    })?;
                sysconf
                    .write_all(format!("user_password={p}\n").as_bytes())
                    .map_err(|e| {
                        Error::Customization(format!(
                            "Failed to write user_password to sysconf.txt: {e}"
                        ))
                    })?;
            }

            if let Some((ssid, _)) = &self.wifi {
                sysconf
                    .write_all(format!("iwd_psk_file={ssid}.psk\n").as_bytes())
                    .map_err(|e| {
                        Error::Customization(format!(
                            "Failed to write iwd_psk_file to sysconf.txt: {e}"
                        ))
                    })?;
            }
        }

        if let Some((ssid, psk)) = &self.wifi {
            let mut wifi_file = boot_root
                .create_file(format!("services/{ssid}.psk").as_str())
                .map_err(|e| Error::Customization(format!("Failed to create iwd_psk_file: {e}")))?;

            wifi_file
                .write_all(
                    format!("[Security]\nPassphrase={psk}\n\n[Settings]\nAutoConnect=true")
                        .as_bytes(),
                )
                .map_err(|e| {
                    Error::Customization(format!("Failed to write to iwd_psk_file: {e}"))
                })?;
        }

        Ok(())
    }

    pub fn update_verify(mut self, verify: bool) -> Self {
        self.verify = verify;
        self
    }

    pub fn update_hostname(mut self, hostname: Option<String>) -> Self {
        self.hostname = hostname;
        self
    }

    pub fn update_timezone(mut self, timezone: Option<String>) -> Self {
        self.timezone = timezone;
        self
    }

    pub fn update_keymap(mut self, k: Option<String>) -> Self {
        self.keymap = k;
        self
    }

    pub fn update_user(mut self, v: Option<(String, String)>) -> Self {
        self.user = v;
        self
    }

    pub fn update_wifi(mut self, v: Option<(String, String)>) -> Self {
        self.wifi = v;
        self
    }
}

impl Default for FlashingSdLinuxConfig {
    fn default() -> Self {
        Self {
            verify: true,
            hostname: Default::default(),
            timezone: Default::default(),
            keymap: Default::default(),
            user: Default::default(),
            wifi: Default::default(),
        }
    }
}
