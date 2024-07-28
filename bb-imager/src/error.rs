//! Command error type for this library

use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("BeagleConnect Freedom Error: {0}")]
    BeagleConnectFreedomError(#[from] crate::bcf::Error),
    #[error("Download Error: {0}")]
    DownloadError(#[from] crate::download::Error),
    #[error("Io Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Image Error: {0}")]
    ImageError(#[from] crate::img::Error),
    #[error("Sd Card Error: {0}")]
    SdCardError(#[from] crate::sd::Error),
    #[error("Zbus Error: {0}")]
    #[cfg(target_os = "linux")]
    DbusClientError(#[from] udisks2::zbus::Error),
    #[error("{0}")]
    CommanError(#[from] crate::common::Error),
}
