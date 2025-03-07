use crate::{Error, Result};

use std::path::Path;

#[cfg(feature = "udev")]
use std::{
    collections::HashMap,
    os::fd::{FromRawFd, IntoRawFd},
};

#[cfg(feature = "udev")]
pub(crate) fn format(dst: &Path) -> Result<()> {
    async fn inner(dst: &Path) -> Result<()> {
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

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()
        .unwrap();
    rt.block_on(async move { inner(dst).await })
}

#[cfg(feature = "udev")]
pub(crate) fn open(dst: &Path) -> Result<std::fs::File> {
    async fn inner(dst: &Path) -> Result<std::fs::File> {
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

        let fd = obj.open_device("rw", Default::default()).await?;

        Ok(unsafe { std::fs::File::from_raw_fd(std::os::fd::OwnedFd::from(fd).into_raw_fd()) })
    }

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()
        .unwrap();
    rt.block_on(async move { inner(dst).await })
}

#[cfg(not(feature = "udev"))]
pub(crate) fn open(dst: &Path) -> Result<std::fs::File> {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(false)
        .open(dst)
        .map_err(Into::into)
}

#[cfg(not(feature = "udev"))]
pub(crate) fn format(dst: &Path) -> Result<()> {
    let output = std::process::Command::new("mkfs.vfat").arg(dst).output()?;

    if output.status.success() {
        Ok(())
    } else {
        Err(Error::FailedToFormat(
            String::from_utf8(output.stderr).unwrap(),
        ))
    }
}
