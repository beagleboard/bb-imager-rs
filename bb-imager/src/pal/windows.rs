use std::fs::{File, OpenOptions};
use std::io::SeekFrom;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::process::Stdio;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Ioctl::{
    FSCTL_ALLOW_EXTENDED_DASD_IO, FSCTL_LOCK_VOLUME, FSCTL_UNLOCK_VOLUME,
};
use windows::Win32::System::IO::DeviceIoControl;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to find the drive {0}")]
    DriveNotFound(String),
    #[error("Drive path is not valid")]
    InvalidDrive,
    #[error("Windows Error: {0}")]
    WindowsError(#[from] windows::core::Error),
}

pub(crate) struct WinDrive {
    drive: File,
    volume: File,
}

impl WinDrive {
    pub(crate) async fn open(path: &str) -> crate::error::Result<Self> {
        let vol_path = physical_drive_to_volume(path)?;
        tracing::info!("Trying to open {vol_path}");
        let volume = open_and_lock_volume(&vol_path)?;

        tracing::info!("Trying to clean {path}");
        diskpart_clean(path).await?;

        tracing::info!("Trying to open {path}");
        let drive = std::fs::OpenOptions::new()
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

impl crate::common::Destination {
    pub(crate) async fn open(&self) -> crate::error::Result<WinDrive> {
        WinDrive::open(&self.path).await
    }
}

fn open_and_lock_volume(path: &str) -> crate::error::Result<File> {
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
        )
        .map_err(Error::from)?;

        DeviceIoControl(
            HANDLE(volume.as_raw_handle()),
            FSCTL_LOCK_VOLUME,
            None,
            0,
            None,
            0,
            None,
            None,
        )
        .map_err(Error::from)?;
    }

    Ok(volume)
}

fn physical_drive_to_volume(drive: &str) -> crate::error::Result<String> {
    let desc = rs_drivelist::drive_list()
        .unwrap()
        .into_iter()
        .find(|x| x.device == drive)
        .ok_or(Error::DriveNotFound(drive.to_string()))?;

    let mount = desc
        .mountpoints
        .get(0)
        .ok_or(Error::DriveNotFound(drive.to_string()))?;

    let mount_path = format!("\\\\.\\{}", mount.path.strip_suffix("\\").unwrap());

    Ok(mount_path)
}

async fn diskpart_clean(path: &str) -> crate::error::Result<()> {
    let disk_num = path
        .strip_prefix("\\\\.\\PhysicalDrive")
        .ok_or(Error::InvalidDrive)?;

    let mut cmd = Command::new("diskpart")
        .stderr(Stdio::null())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()?;

    let mut stdin = cmd.stdin.take().unwrap();
    stdin.write_all(b"select disk ").await?;
    stdin.write_all(disk_num.as_bytes()).await?;
    stdin.write_all(b"\n").await?;
    stdin.write_all(b"clean\n").await?;
    stdin.write_all(b"rescan\n").await?;
    stdin.write_all(b"exit\n").await?;

    drop(stdin);

    cmd.wait().await?;

    Ok(())
}

impl std::io::Read for WinDrive {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.drive.read(buf)
    }
}

impl std::io::Write for WinDrive {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.drive.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.drive.flush()
    }
}

impl std::io::Seek for WinDrive {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.drive.seek(pos)
    }
}
