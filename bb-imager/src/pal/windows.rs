use std::{
    io::SeekFrom,
    os::windows::io::AsRawHandle,
    pin::Pin,
    process::Stdio,
    task::{Context, Poll},
};
use thiserror::Error;
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncWriteExt, ReadBuf},
    process::Command,
};
use windows::Win32::{
    Foundation::HANDLE,
    System::Ioctl::{FSCTL_ALLOW_EXTENDED_DASD_IO, FSCTL_LOCK_VOLUME, FSCTL_UNLOCK_VOLUME},
    System::IO::DeviceIoControl,
};

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

pub(crate) struct WinDriveStd {
    drive: std::fs::File,
    volume: File,
}

impl WinDrive {
    pub(crate) async fn open(path: &str) -> crate::error::Result<Self> {
        let vol_path = physical_drive_to_volume(path)?;
        tracing::info!("Trying to open {vol_path}");
        let volume = open_and_lock_volume(&vol_path).await?;

        tracing::info!("Trying to clean {path}");
        diskpart_clean(path).await?;

        tracing::info!("Trying to open {path}");
        let drive = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(0x20000000)
            .open(path)
            .await?;

        Ok(Self { drive, volume })
    }

    pub(crate) async fn into_std(self) -> WinDriveStd {
        WinDriveStd {
            volume: self.volume,
            drive: self.drive.into_std().await,
        }
    }
}

impl Drop for WinDriveStd {
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

async fn open_and_lock_volume(path: &str) -> crate::error::Result<File> {
    let volume = OpenOptions::new().read(true).write(true).open(path).await?;

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

impl tokio::io::AsyncRead for WinDrive {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.drive).poll_read(cx, buf)
    }
}

impl tokio::io::AsyncWrite for WinDrive {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.drive).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.drive).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.drive).poll_shutdown(cx)
    }
}

impl tokio::io::AsyncSeek for WinDrive {
    fn start_seek(mut self: Pin<&mut Self>, position: SeekFrom) -> std::io::Result<()> {
        Pin::new(&mut self.drive).start_seek(position)
    }

    fn poll_complete(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<u64>> {
        Pin::new(&mut self.drive).poll_complete(cx)
    }
}

impl std::io::Read for WinDriveStd {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.drive.read(buf)
    }
}

impl std::io::Write for WinDriveStd {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.drive.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.drive.flush()
    }
}

impl std::io::Seek for WinDriveStd {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.drive.seek(pos)
    }
}
