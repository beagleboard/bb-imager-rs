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
    async fn format_inner(dst: &Path) -> io::Result<()> {
        let dbus_client = udisks2::Client::new().await.map_err(io::Error::other)?;

        let devs = dbus_client
            .manager()
            .resolve_device(
                HashMap::from([("path", dst.to_str().unwrap().into())]),
                HashMap::new(),
            )
            .await
            .map_err(io::Error::other)?;

        let block = devs
            .first()
            .ok_or(io::Error::new(
                io::ErrorKind::NotFound,
                "Block device not found",
            ))?
            .to_owned();

        let obj = dbus_client
            .object(block)
            .expect("Unexpected error")
            .block()
            .await
            .map_err(io::Error::other)?;

        obj.format(
            "vfat",
            HashMap::from([("update-partition-type", true.into())]),
        )
        .await
        .map_err(io::Error::other)?;

        Ok(())
    }

    format_inner(dst)
        .await
        .map_err(|source| Error::FailedToFormat { source })
}

#[cfg(feature = "udev")]
pub(crate) async fn open(dst: &Path) -> Result<LinuxDrive> {
    async fn open_inner(dst: &Path) -> anyhow::Result<LinuxDrive> {
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
            .ok_or(anyhow::anyhow!("Block device not found",))?
            .to_owned();

        let obj = dbus_client
            .object(block)
            .expect("Unexpected error")
            .block()
            .await?;

        let fd = obj
            .open_device("rw", HashMap::from([("flags", libc::O_DIRECT.into())]))
            .await?;
        let file =
            unsafe { std::fs::File::from_raw_fd(std::os::fd::OwnedFd::from(fd).into_raw_fd()) };

        Ok(LinuxDrive {
            file,
            drive: dst.to_path_buf(),
        })
    }

    open_inner(dst)
        .await
        .map_err(|e| Error::FailedToOpenDestination { source: e })
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
    async fn format_inner(dst: &Path) -> io::Result<()> {
        let output = tokio::process::Command::new("mkfs.vfat")
            .arg(dst)
            .output()
            .await?;

        if output.status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!("Status: {}", output.status)))
        }
    }

    format_inner(dst)
        .await
        .map_err(|source| Error::FailedToFormat { source })
}

#[derive(Debug)]
pub(crate) struct LinuxDrive {
    file: std::fs::File,
    drive: PathBuf,
}

#[cfg(feature = "udev")]
impl Eject for LinuxDrive {
    fn eject(self) -> io::Result<()> {
        async fn inner(dst: PathBuf) -> io::Result<()> {
            let dbus_client = udisks2::Client::new().await.map_err(io::Error::other)?;

            let devs = dbus_client
                .manager()
                .resolve_device(
                    HashMap::from([("path", dst.to_str().unwrap().into())]),
                    HashMap::new(),
                )
                .await
                .map_err(io::Error::other)?;

            let obj_path = devs
                .first()
                .ok_or(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Block device not found",
                ))?
                .to_owned();

            let block = dbus_client
                .object(obj_path)
                .expect("Unexpected error")
                .block()
                .await
                .map_err(io::Error::other)?;

            dbus_client
                .object(block.drive().await.map_err(io::Error::other)?)
                .expect("Unexpected error")
                .drive()
                .await
                .map_err(io::Error::other)?
                .eject(HashMap::new())
                .await
                .map_err(io::Error::other)?;

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
