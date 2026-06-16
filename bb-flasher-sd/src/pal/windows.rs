use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::path::Path;
use std::process::Stdio;

use windows::Win32::{
    Foundation::HANDLE,
    System::IO::DeviceIoControl,
    System::Ioctl::{FSCTL_ALLOW_EXTENDED_DASD_IO, FSCTL_LOCK_VOLUME, FSCTL_UNLOCK_VOLUME},
};

use crate::{Error, Result};

#[derive(Debug)]
pub(crate) struct WinDrive {
    drive: File,
    volume: Option<File>,
}

const FILE_FLAG_WRITE_THROUGH: u32 = 0x80000000;
const FILE_FLAG_NO_BUFFERING: u32 = 0x20000000;

impl WinDrive {
    pub(crate) fn open(path: &Path) -> Result<Self> {
        tracing::info!("Trying to find {}", path.display());
        let vol_path = physical_drive_to_volume(path)?;

        let volume = if let Some(vol_path) = vol_path {
            tracing::info!("Trying to open {vol_path}");
            Some(open_and_lock_volume(&vol_path).ok_or(Error::DriveNotFound)?)
        } else {
            None
        };

        tracing::info!("Trying to clean {:?}", path);
        diskpart_clean(path)?;

        tracing::info!("Trying to open {:?}", path);
        let drive = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(FILE_FLAG_WRITE_THROUGH | FILE_FLAG_NO_BUFFERING)
            .open(path)?;

        Ok(Self { drive, volume })
    }
}

impl Drop for WinDrive {
    fn drop(&mut self) {
        if let Some(volume) = &self.volume {
            let _ = unsafe {
                DeviceIoControl(
                    HANDLE(volume.as_raw_handle()),
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
}

fn open_and_lock_volume(path: &str) -> Option<File> {
    let volume = OpenOptions::new().read(true).write(true).open(path).ok()?;

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
        .ok()?;

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
        .ok()?;
    }

    Some(volume)
}

fn physical_drive_to_volume(drive: &Path) -> Result<Option<String>> {
    let desc = bb_drivelist::drive_list()
        .expect("Unexpected error")
        .into_iter()
        .find(|x| x.device == drive.to_str().unwrap())
        .ok_or(Error::DriveNotFound)?;

    tracing::info!("Drive desc {:#?}", desc);

    if let Some(mount) = desc.mountpoints.first() {
        let mount_path = format!(
            "\\\\.\\{}",
            mount
                .path
                .strip_suffix("\\")
                .ok_or(io::Error::new(io::ErrorKind::NotFound, "Drive not found"))?
        );

        Ok(Some(mount_path))
    } else {
        Ok(None)
    }
}

fn diskpart_clean(path: &Path) -> Result<()> {
    let disk_num = path
        .to_str()
        .unwrap()
        .strip_prefix("\\\\.\\PhysicalDrive")
        .ok_or(io::Error::new(io::ErrorKind::NotFound, "Drive not found"))?;

    let resp = std::process::Command::new("powershell")
        .args(&[
            "Clear-Disk",
            "-Number",
            disk_num,
            "-RemoveData",
            "-Confirm:$false",
        ])
        .output()?;
    tracing::info!("Disk Clear Response: {:#?}", resp);

    if resp.status.success() {
        Ok(())
    } else {
        Err(Error::WindowsCleanError(resp))
    }
}

fn diskpart_format(path: &Path) -> io::Result<()> {
    let disk_num = path
        .to_str()
        .unwrap()
        .strip_prefix("\\\\.\\PhysicalDrive")
        .ok_or(io::Error::new(io::ErrorKind::NotFound, "Drive not found"))?;

    let mut cmd = std::process::Command::new("diskpart")
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

    let status = cmd.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("Status: {status}")))
    }
}

impl Read for WinDrive {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.drive.read(buf)
    }
}

impl Write for WinDrive {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.drive.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.drive.flush()
    }
}

impl Seek for WinDrive {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.drive.seek(pos)
    }
}

/// TODO: Implement real eject
impl crate::helpers::Eject for WinDrive {
    fn eject(self) -> io::Result<()> {
        self.drive.sync_all()?;
        Ok(())
    }
}

pub(crate) fn format(dst: &Path) -> Result<()> {
    diskpart_format(dst).map_err(|source| Error::FailedToFormat { source })
}

pub(crate) fn open(dst: &Path) -> Result<WinDrive> {
    WinDrive::open(dst).map_err(|_| Error::DriveNotFound)
}
