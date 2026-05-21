use crate::helpers::Eject;
use crate::{Error, Result};

use std::{
    io,
    path::{Path, PathBuf},
    pin::Pin,
    task::{Context, Poll},
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
            unsafe { tokio::fs::File::from_raw_fd(std::os::fd::OwnedFd::from(fd).into_raw_fd()) };

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
        .await?;

    Ok(LinuxDrive {
        file,
        drive: dst.to_path_buf(),
    })
}

#[cfg(not(feature = "udev"))]
pub(crate) async fn format(dst: &Path) -> Result<()> {
    let sd = tokio::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(false)
        .open(dst)
        .await?
        .into_std()
        .await;

    tokio::task::spawn_blocking(|| fatfs::format_volume(sd, fatfs::FormatVolumeOptions::default()))
        .await
        .unwrap()
        .map_err(|source| Error::FailedToFormat { source })
}

#[derive(Debug)]
pub(crate) struct LinuxDrive {
    file: tokio::fs::File,
    drive: PathBuf,
}

#[cfg(feature = "udev")]
impl Eject for LinuxDrive {
    async fn eject(self) -> io::Result<()> {
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

        let _ = self.file.sync_all().await?;
        let dst = self.drive.clone();

        std::mem::drop(self);

        inner(dst).await.map_err(io::Error::other)
    }
}

#[cfg(not(feature = "udev"))]
impl Eject for LinuxDrive {
    async fn eject(self) -> std::io::Result<()> {
        let _ = self.file.sync_all().await;
        let drive = self.drive.clone();
        std::mem::drop(self);

        let output = tokio::process::Command::new("eject")
            .arg(drive)
            .output()
            .await?;

        if output.status.success() {
            Ok(())
        } else {
            Err(std::io::Error::other(
                String::from_utf8(output.stderr).unwrap(),
            ))
        }
    }
}
impl tokio::io::AsyncRead for LinuxDrive {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.file).poll_read(cx, buf)
    }
}

impl tokio::io::AsyncSeek for LinuxDrive {
    fn start_seek(mut self: Pin<&mut Self>, position: io::SeekFrom) -> io::Result<()> {
        Pin::new(&mut self.file).start_seek(position)
    }

    fn poll_complete(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
        Pin::new(&mut self.file).poll_complete(cx)
    }
}

impl tokio::io::AsyncWrite for LinuxDrive {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.file).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.file).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.file).poll_shutdown(cx)
    }
}
