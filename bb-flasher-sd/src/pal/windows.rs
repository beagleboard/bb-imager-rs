use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    os::windows::{fs::OpenOptionsExt, io::AsRawHandle},
    path::Path,
    process::{Command, Stdio},
};
use windows::Win32::{
    Foundation::HANDLE,
    System::IO::DeviceIoControl,
    System::Ioctl::{FSCTL_ALLOW_EXTENDED_DASD_IO, FSCTL_LOCK_VOLUME, FSCTL_UNLOCK_VOLUME},
};

use crate::{Error, Result};

pub(crate) struct WinDrive {
    drive: File,
    volume: File,
}

impl WinDrive {
    pub(crate) fn open(path: &Path) -> Result<Self> {
        let vol_path = physical_drive_to_volume(path)?;
        tracing::info!("Trying to open {vol_path}");
        let volume = open_and_lock_volume(&vol_path)?;

        tracing::info!("Trying to clean {:?}", path);
        diskpart_clean(path)?;

        tracing::info!("Trying to open {:?}", path);
        let drive = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(0x20000000)
            .open(path)?;

        Ok(Self { drive, volume })
    }
}

impl Drop for WinDrive {
    fn drop(&mut self) {
        let _ = unsafe {
            DeviceIoControl(
                HANDLE(self.volume.as_raw_handle()),
                FSCTL_UNLOCK_VOLUME,
                None,
                0,
                None,
                0,
                None,
                None,
            )
        };
    }
}

fn open_and_lock_volume(path: &str) -> Result<File> {
    let volume = OpenOptions::new().read(true).write(true).open(path)?;

    unsafe {
        DeviceIoControl(
            HANDLE(volume.as_raw_handle()),
            FSCTL_ALLOW_EXTENDED_DASD_IO,
            None,
            0,
            None,
            0,
            None,
            None,
        )?;

        DeviceIoControl(
            HANDLE(volume.as_raw_handle()),
            FSCTL_LOCK_VOLUME,
            None,
            0,
            None,
            0,
            None,
            None,
        )?;
    }

    Ok(volume)
}

fn physical_drive_to_volume(drive: &Path) -> Result<String> {
    let desc = bb_drivelist::drive_list()
        .expect("Unexpected error")
        .into_iter()
        .find(|x| x.device == drive.to_str().unwrap())
        .ok_or(Error::DriveNotFound(drive.to_string_lossy().to_string()))?;

    let mount = desc
        .mountpoints
        .first()
        .ok_or(Error::DriveNotFound(drive.to_string_lossy().to_string()))?;

    let mount_path = format!(
        "\\\\.\\{}",
        mount.path.strip_suffix("\\").ok_or(Error::InvalidDrive)?
    );

    Ok(mount_path)
}

fn diskpart_clean(path: &Path) -> Result<()> {
    let disk_num = path
        .to_str()
        .unwrap()
        .strip_prefix("\\\\.\\PhysicalDrive")
        .ok_or(Error::InvalidDrive)?;

    let mut cmd = Command::new("diskpart")
        .stderr(Stdio::null())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()?;

    let mut stdin = cmd.stdin.take().expect("Failed to get stdin");
    stdin.write_all(b"select disk ")?;
    stdin.write_all(disk_num.as_bytes())?;
    stdin.write_all(b"\n")?;
    stdin.write_all(b"clean\n")?;
    stdin.write_all(b"rescan\n")?;
    stdin.write_all(b"exit\n")?;

    drop(stdin);

    cmd.wait()?;

    Ok(())
}

fn diskpart_format(path: &Path) -> Result<()> {
    let disk_num = path
        .to_str()
        .unwrap()
        .strip_prefix("\\\\.\\PhysicalDrive")
        .ok_or(Error::InvalidDrive)?;

    let mut cmd = Command::new("diskpart")
        .stderr(Stdio::null())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()?;

    let mut stdin = cmd.stdin.take().expect("Failed to get stdin");
    stdin.write_all(b"select disk ")?;
    stdin.write_all(disk_num.as_bytes())?;
    stdin.write_all(b"\n")?;
    stdin.write_all(b"clean\n")?;
    stdin.write_all(b"create partition primary\n")?;
    stdin.write_all(b"format quick fs=fat32\n")?;
    stdin.write_all(b"assign\n")?;
    stdin.write_all(b"exit\n")?;

    drop(stdin);

    cmd.wait()?;

    Ok(())
}

impl Read for WinDrive {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.drive.read(buf)
    }
}

impl Write for WinDrive {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.drive.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.drive.flush()
    }
}

impl Seek for WinDrive {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.drive.seek(pos)
    }
}

pub(crate) fn format(dst: &Path) -> Result<()> {
    tracing::debug!("Trying to format {:?}", dst);
    diskpart_format(dst).map_err(|e| Error::FailedToFormat(e.to_string()))
}

pub(crate) fn open(dst: &Path) -> Result<WinDrive> {
    WinDrive::open(dst)
}
