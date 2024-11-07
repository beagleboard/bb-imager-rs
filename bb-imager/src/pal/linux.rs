use thiserror::Error;

use std::{
    collections::HashMap,
    os::fd::{FromRawFd, IntoRawFd},
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to open destination {0}")]
    FailedToOpenDestionation(String),
    #[error("Zbus Error: {0}")]
    #[cfg(target_os = "linux")]
    DbusClientError(#[from] udisks2::zbus::Error),
}

impl crate::common::Destination {
    pub async fn format(&self) -> crate::error::Result<()> {
        if let Self::SdCard { path, .. } = self {
            let dbus_client = udisks2::Client::new().await.map_err(Error::from)?;

            let devs = dbus_client
                .manager()
                .resolve_device(
                    HashMap::from([("path", path.as_str().into())]),
                    HashMap::new(),
                )
                .await
                .map_err(Error::from)?;

            let block = devs
                .first()
                .ok_or(Error::FailedToOpenDestionation(path.clone()))?
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
        } else {
            unreachable!()
        }
    }

    pub async fn open(&self) -> crate::error::Result<tokio::fs::File> {
        if let Self::SdCard { path, .. } = self {
            let dbus_client = udisks2::Client::new().await.map_err(Error::from)?;

            let devs = dbus_client
                .manager()
                .resolve_device(
                    HashMap::from([("path", path.as_str().into())]),
                    HashMap::new(),
                )
                .await
                .map_err(Error::from)?;

            let block = devs
                .first()
                .ok_or(Error::FailedToOpenDestionation(path.clone()))?
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

            Ok(unsafe {
                tokio::fs::File::from_raw_fd(std::os::fd::OwnedFd::from(fd).into_raw_fd())
            })
        } else {
            unreachable!()
        }
    }
}
