use std::io::{Read, Seek, SeekFrom, Write};

use crate::{Error, Result};

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
/// Post install customization options
pub struct Customization {
    pub hostname: Option<String>,
    pub timezone: Option<String>,
    pub keymap: Option<String>,
    pub user: Option<(String, String)>,
    pub wifi: Option<(String, String)>,
}

impl Customization {
    pub(crate) fn customize(&self, mut dst: impl Write + Seek + Read) -> Result<()> {
        if !self.has_customization() {
            return Ok(());
        }

        let boot_partition = {
            let mbr = mbrman::MBR::read_from(&mut dst, 512)
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

        let mut conf = boot_root
            .create_file("sysconf.txt")
            .map_err(|e| Error::Customization(format!("Failed to create sysconf.txt: {e}")))?;
        conf.seek(SeekFrom::End(0)).map_err(|e| {
            Error::Customization(format!("Failed to seek to end of sysconf.txt: {e}"))
        })?;

        if let Some(h) = &self.hostname {
            sysconf_w(&mut conf, &format!("hostname={h}\n"), "hostname")?;
        }

        if let Some(tz) = &self.timezone {
            sysconf_w(&mut conf, &format!("timezone={tz}\n"), "timezone")?;
        }

        if let Some(k) = &self.keymap {
            sysconf_w(&mut conf, &format!("keymap={k}\n"), "keymap")?;
        }

        if let Some((u, p)) = &self.user {
            sysconf_w(&mut conf, &format!("user_name={u}\n"), "user_name")?;
            sysconf_w(&mut conf, &format!("user_password={p}\n"), "user_password")?;
        }

        if let Some((ssid, psk)) = &self.wifi {
            sysconf_w(
                &mut conf,
                &format!("iwd_psk_file={ssid}.psk\n"),
                "iwd_psk_file",
            )?;

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

    pub(crate) const fn has_customization(&self) -> bool {
        self.hostname.is_some()
            || self.timezone.is_some()
            || self.keymap.is_some()
            || self.user.is_some()
            || self.wifi.is_some()
    }
}

fn sysconf_w(mut sysconf: impl Write, data: &str, field: &str) -> Result<()> {
    sysconf.write_all(data.as_bytes()).map_err(|e| {
        Error::Customization(format!("Failed to write {field} to sysconf.txt: {e}"))
    })?;
    Ok(())
}
