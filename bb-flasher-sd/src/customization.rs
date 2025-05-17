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
    pub ssh: Option<String>,
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
            sysconf_w(&mut conf, &format!("hostname={h}\n"))?;
        }

        if let Some(tz) = &self.timezone {
            sysconf_w(&mut conf, &format!("timezone={tz}\n"))?;
        }

        if let Some(k) = &self.keymap {
            sysconf_w(&mut conf, &format!("keymap={k}\n"))?;
        }

        if let Some((u, p)) = &self.user {
            sysconf_w(&mut conf, &format!("user_name={u}\n"))?;
            sysconf_w(&mut conf, &format!("user_password={p}\n"))?;
        }

        if let Some(x) = &self.ssh {
            sysconf_w(&mut conf, &format!("user_authorized_key={x}"))?;
        }

        if let Some((ssid, psk)) = &self.wifi {
            sysconf_w(&mut conf, &format!("iwd_psk_file={ssid}.psk\n"))?;

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

fn sysconf_w(mut sysconf: impl Write, data: &str) -> Result<()> {
    sysconf
        .write_all(data.as_bytes())
        .map_err(|e| Error::Customization(format!("Failed to write {data} to sysconf.txt: {e}")))?;
    Ok(())
}
