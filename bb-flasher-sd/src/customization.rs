use crate::{Error, Result};
use fatfs::FileSystem;
use fscommon::{BufStream, StreamSlice};
use std::io::{Read, Seek, SeekFrom, Write};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Customization {
    Sysconf(SysconfCustomization),
    GenericFile(GenericFileCustomization),
}

impl Customization {
    pub(crate) fn customize(&self, dst: impl Write + Seek + Read + std::fmt::Debug) -> Result<()> {
        match self {
            Self::Sysconf(x) => x.customize(dst),
            Self::GenericFile(x) => x.customize(dst),
        }
    }

    pub(crate) fn validate(&self) -> bool {
        match self {
            Self::Sysconf(x) => x.validate(),
            Self::GenericFile(x) => x.validate(),
        }
    }
}

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
/// Post install customization options
pub struct SysconfCustomization {
    pub hostname: Option<Box<str>>,
    pub timezone: Option<Box<str>>,
    pub keymap: Option<Box<str>>,
    pub user: Option<(Box<str>, Box<str>)>,
    pub wifi: Option<(Box<str>, Box<str>)>,
    pub ssh: Option<Box<str>>,
    pub usb_enable_dhcp: Option<bool>,
}

impl SysconfCustomization {
    pub(crate) fn customize(
        &self,
        mut dst: impl Write + Seek + Read + std::fmt::Debug,
    ) -> Result<()> {
        if !self.has_customization() {
            return Ok(());
        }

        let boot_partition = customization_partition(&mut dst)?;
        let boot_root = boot_partition.root_dir();

        let mut conf = boot_root
            .create_file("sysconf.txt")
            .map_err(|source| Error::SysconfCreateFail { source })?;
        conf.seek(SeekFrom::End(0))
            .expect("Failed to seek to end of sysconf.txt");

        if let Some(h) = &self.hostname {
            sysconf_w(&mut conf, "hostname", h)?;
        }

        if let Some(tz) = &self.timezone {
            sysconf_w(&mut conf, "timezone", tz)?;
        }

        if let Some(k) = &self.keymap {
            sysconf_w(&mut conf, "keymap", k)?;
        }

        if let Some((u, p)) = &self.user {
            sysconf_w(&mut conf, "user_name", u)?;
            sysconf_w(&mut conf, "user_password", p)?;
        }

        if let Some(x) = &self.ssh {
            sysconf_w(&mut conf, "user_authorized_key", x)?;
        }

        if Some(true) == self.usb_enable_dhcp {
            sysconf_w(&mut conf, "usb_enable_dhcp", "yes")?;
        }

        if let Some((ssid, psk)) = &self.wifi {
            let mut wifi_file = boot_root
                .create_file(format!("services/{ssid}.psk").as_str())
                .map_err(|e| Error::WifiSetupFail { source: e })?;

            wifi_file
                .write_all(
                    format!("[Security]\nPassphrase={psk}\n\n[Settings]\nAutoConnect=true")
                        .as_bytes(),
                )
                .map_err(|e| Error::WifiSetupFail { source: e })?;

            sysconf_w(&mut conf, "iwd_psk_file", &format!("{ssid}.psk"))?;
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

    pub(crate) fn validate(&self) -> bool {
        if let Some((x, _)) = &self.user {
            x.as_ref() != "root"
        } else {
            true
        }
    }
}

fn sysconf_w(mut sysconf: impl Write, key: &'static str, value: &str) -> Result<()> {
    sysconf
        .write_all(format!("{key}={value}\n").as_bytes())
        .map_err(|e| Error::SysconfWriteFail {
            source: e,
            field: key,
        })
}

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
/// Post install customization options
pub struct GenericFileCustomization {
    pub file_name: Box<str>,
    pub file_content: Option<Box<str>>,
}

impl GenericFileCustomization {
    pub(crate) fn customize(
        &self,
        mut dst: impl Write + Seek + Read + std::fmt::Debug,
    ) -> Result<()> {
        let boot_partition = customization_partition(&mut dst)?;
        let boot_root = boot_partition.root_dir();

        let mut file = boot_root.create_file(&self.file_name).map_err(|source| {
            Error::GenericFileCreateFail {
                source,
                file: self.file_name.clone(),
            }
        })?;
        file.seek(SeekFrom::End(0))
            .unwrap_or_else(|_| panic!("Failed to seek to end of {}", self.file_name));
        if let Some(content) = &self.file_content {
            file.write_all(content.as_bytes())
                .map_err(|source| Error::GenericFileWriteFail {
                    source,
                    file: self.file_name.clone(),
                })?;
        }

        Ok(())
    }

    pub(crate) fn validate(&self) -> bool {
        !self.file_name.is_empty()
    }
}

fn customization_partition<T: Write + Seek + Read + std::fmt::Debug>(
    mut dst: T,
) -> Result<FileSystem<BufStream<StreamSlice<T>>>> {
    // First try GPT partition table. If that fails, try MBR
    let (start_offset, end_offset) = if let Ok(disk) = gpt::GptConfig::new()
        .writable(false)
        .open_from_device(&mut dst)
    {
        // FIXME: Add better partition lookup
        let partition_2 = disk.partitions().get(&2).unwrap();

        let start_offset: u64 = partition_2.first_lba * gpt::disk::DEFAULT_SECTOR_SIZE.as_u64();
        let end_offset: u64 = partition_2.last_lba * gpt::disk::DEFAULT_SECTOR_SIZE.as_u64();

        (start_offset, end_offset)
    } else {
        let mbr =
            mbrman::MBRHeader::read_from(&mut dst).map_err(|_| Error::InvalidPartitionTable)?;

        let boot_part = mbr.get(1).ok_or(Error::InvalidPartitionTable)?;
        let start_offset: u64 = (boot_part.starting_lba * 512).into();
        let end_offset: u64 = start_offset + u64::from(boot_part.sectors) * 512;

        (start_offset, end_offset)
    };
    let slice = StreamSlice::new(dst, start_offset, end_offset)
        .map_err(|_| Error::InvalidPartitionTable)?;
    let boot_stream = BufStream::new(slice);
    FileSystem::new(boot_stream, fatfs::FsOptions::new()).map_err(|_| Error::InvalidBootPartition)
}
