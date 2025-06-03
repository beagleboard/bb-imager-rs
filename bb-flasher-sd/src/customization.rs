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
    pub usb_enable_dhcp: Option<bool>,
}

impl Customization {
    pub(crate) fn customize(
        &self,
        mut dst: impl Write + Seek + Read + std::fmt::Debug,
    ) -> Result<()> {
        if !self.has_customization() {
            return Ok(());
        }

        let boot_partition = {
            let (start_off, end_off) = customization_partition(&mut dst)?;
            let slice = fscommon::StreamSlice::new(dst, start_off, end_off)
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

        if Some(true) == self.usb_enable_dhcp {
            sysconf_w(&mut conf, "usb_enable_dhcp=yes")?;
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

    pub(crate) fn has_customization(&self) -> bool {
        self.hostname.is_some()
            || self.timezone.is_some()
            || self.keymap.is_some()
            || self.user.is_some()
            || self.wifi.is_some()
            || self.ssh.is_some()
            || self.usb_enable_dhcp == Some(true)
    }
}

fn sysconf_w(mut sysconf: impl Write, data: &str) -> Result<()> {
    sysconf
        .write_all(data.as_bytes())
        .map_err(|e| Error::Customization(format!("Failed to write {data} to sysconf.txt: {e}")))?;
    Ok(())
}

fn customization_partition(
    mut dst: impl Write + Seek + Read + std::fmt::Debug,
) -> Result<(u64, u64)> {
    // First try GPT partition table. If that fails, try MBR
    if let Ok(disk) = gpt::GptConfig::new()
        .writable(false)
        .open_from_device(&mut dst)
    {
        // FIXME: Add better partition lookup
        let partition_2 = disk.partitions().get(&2).unwrap();

        let start_offset: u64 = partition_2.first_lba * gpt::disk::DEFAULT_SECTOR_SIZE.as_u64();
        let end_offset: u64 = partition_2.last_lba * gpt::disk::DEFAULT_SECTOR_SIZE.as_u64();

        Ok((start_offset, end_offset))
    } else {
        let mbr = mbrman::MBRHeader::read_from(&mut dst)
            .map_err(|e| Error::Customization(format!("Failed to read mbr: {e}")))?;

        let boot_part = mbr.get(1).ok_or(Error::Customization(
            "Failed to get boot partition".to_string(),
        ))?;
        let start_offset: u64 = (boot_part.starting_lba * 512).into();
        let end_offset: u64 = start_offset + u64::from(boot_part.sectors) * 512;

        Ok((start_offset, end_offset))
    }
}
