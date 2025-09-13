use crate::{Error, Result, helpers::Eject};

use std::{
    io,
    path::{Path, PathBuf},
};

#[cfg(feature = "udev")]
use std::{
    collections::HashMap,
    os::fd::{FromRawFd, IntoRawFd},
};

#[cfg(feature = "udev")]
pub(crate) async fn format(dst: &Path) -> Result<()> {
    let dbus_client = udisks2::Client::new().await.map_err(Error::from)?;

    let devs = dbus_client
        .manager()
        .resolve_device(
            HashMap::from([("path", dst.to_str().unwrap().into())]),
            HashMap::new(),
        )
        .await
        .map_err(Error::from)?;

    let block = devs
        .first()
        .ok_or(Error::FailedToOpenDestination(
            dst.to_string_lossy().to_string(),
        ))?
        .to_owned();

    let obj = dbus_client
        .object(block)
        .expect("Unexpected error")
        .block()
        .await
        .map_err(Error::from)?;

    obj.format(
        "vfat",
        HashMap::from([("update-partition-type", true.into())]),
    )
    .await
    .map_err(Error::from)?;

    Ok(())
}

#[cfg(feature = "udev")]
pub(crate) async fn open(dst: &Path) -> Result<LinuxDrive> {
    let dbus_client = udisks2::Client::new().await?;

    let devs = dbus_client
        .manager()
        .resolve_device(
            HashMap::from([("path", dst.to_str().unwrap().into())]),
            HashMap::new(),
        )
        .await?;

    let block = devs
        .first()
        .ok_or(Error::FailedToOpenDestination(
            dst.to_string_lossy().to_string(),
        ))?
        .to_owned();

    let obj = dbus_client
        .object(block)
        .expect("Unexpected error")
        .block()
        .await?;

    let fd = obj
        .open_device("rw", HashMap::from([("flags", libc::O_DIRECT.into())]))
        .await?;
    let file = unsafe { std::fs::File::from_raw_fd(std::os::fd::OwnedFd::from(fd).into_raw_fd()) };

    Ok(LinuxDrive {
        file,
        drive: dst.to_path_buf(),
    })
}

#[cfg(not(feature = "udev"))]
pub(crate) async fn open(dst: &Path) -> Result<LinuxDrive> {
    let file = tokio::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(false)
        .custom_flags(libc::O_DIRECT)
        .open(dst)
        .await?
        .into_std()
        .await;

    Ok(LinuxDrive {
        file,
        drive: dst.to_path_buf(),
    })
}

#[cfg(not(feature = "udev"))]
pub(crate) async fn format(dst: &Path) -> Result<()> {
    let output = tokio::process::Command::new("mkfs.vfat")
        .arg(dst)
        .output()
        .await?;

    if output.status.success() {
        Ok(())
    } else {
        Err(Error::FailedToFormat(
            String::from_utf8(output.stderr).unwrap(),
        ))
    }
}

#[derive(Debug)]
pub(crate) struct LinuxDrive {
    file: std::fs::File,
    drive: PathBuf,
}

#[cfg(feature = "udev")]
impl Eject for LinuxDrive {
    fn eject(self) -> io::Result<()> {
        async fn inner(dst: PathBuf) -> Result<()> {
            let dbus_client = udisks2::Client::new().await?;

            let devs = dbus_client
                .manager()
                .resolve_device(
                    HashMap::from([("path", dst.to_str().unwrap().into())]),
                    HashMap::new(),
                )
                .await?;

            let obj_path = devs
                .first()
                .ok_or(Error::FailedToOpenDestination(
                    dst.to_string_lossy().to_string(),
                ))?
                .to_owned();

            let block = dbus_client
                .object(obj_path)
                .expect("Unexpected error")
                .block()
                .await?;

            dbus_client
                .object(block.drive().await?)
                .expect("Unexpected error")
                .drive()
                .await?
                .eject(HashMap::new())
                .await?;

            Ok(())
        }

        let _ = self.file.sync_all();
        let dst = self.drive.clone();

        std::mem::drop(self);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap();
        rt.block_on(async move { inner(dst).await })
            .map_err(io::Error::other)
    }
}

#[cfg(not(feature = "udev"))]
impl Eject for LinuxDrive {
    fn eject(self) -> std::io::Result<()> {
        let _ = self.file.sync_all();
        let drive = self.drive.clone();
        std::mem::drop(self);

        let output = std::process::Command::new("eject").arg(drive).output()?;

        if output.status.success() {
            Ok(())
        } else {
            Err(std::io::Error::other(
                String::from_utf8(output.stderr).unwrap(),
            ))
        }
    }
}

impl io::Read for LinuxDrive {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }
}

impl io::Seek for LinuxDrive {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.file.seek(pos)
    }
}

impl io::Write for LinuxDrive {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
