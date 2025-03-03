use thiserror::Error;

#[cfg(feature = "udisks2")]
use std::{
    collections::HashMap,
    os::fd::{FromRawFd, IntoRawFd},
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to open destination {0}")]
    FailedToOpenDestionation(String),
    #[error("Formatting failed: {0}")]
    FailedToFormat(String),
    #[error("Udisks2 Error: {0}")]
    #[cfg(feature = "udisks2")]
    Udisks(#[from] udisks2::Error),
}

#[cfg(feature = "udisks2")]
pub(crate) async fn format_sd(dst: &str) -> crate::error::Result<()> {
    let dbus_client = udisks2::Client::new().await.map_err(Error::from)?;

    let devs = dbus_client
        .manager()
        .resolve_device(HashMap::from([("path", dst.into())]), HashMap::new())
        .await
        .map_err(Error::from)?;

    let block = devs
        .first()
        .ok_or(Error::FailedToOpenDestionation(dst.to_string()))?
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

#[cfg(feature = "udisks2")]
pub(crate) async fn open_sd(dst: &str) -> crate::error::Result<tokio::fs::File> {
    let dbus_client = udisks2::Client::new().await.map_err(Error::from)?;

    let devs = dbus_client
        .manager()
        .resolve_device(HashMap::from([("path", dst.into())]), HashMap::new())
        .await
        .map_err(Error::from)?;

    let block = devs
        .first()
        .ok_or(Error::FailedToOpenDestionation(dst.to_string()))?
        .to_owned();

    let obj = dbus_client
        .object(block)
        .expect("Unexpected error")
        .block()
        .await
        .map_err(Error::from)?;

    let fd = obj
        .open_device("rw", Default::default())
        .await
        .map_err(Error::from)?;

    Ok(unsafe { tokio::fs::File::from_raw_fd(std::os::fd::OwnedFd::from(fd).into_raw_fd()) })
}

#[cfg(not(feature = "udisks2"))]
pub(crate) async fn open_sd(dst: &str) -> crate::error::Result<tokio::fs::File> {
    tokio::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(false)
        .open(dst)
        .await
        .map_err(Into::into)
}

#[cfg(not(feature = "udisks2"))]
pub(crate) async fn format_sd(dst: &str) -> crate::error::Result<()> {
    let output = tokio::process::Command::new("mkfs.vfat")
        .arg(dst)
        .output()
        .await?;

    if output.status.success() {
        Ok(())
    } else {
        Err(Error::FailedToFormat(String::from_utf8(output.stderr).unwrap()).into())
    }
}
