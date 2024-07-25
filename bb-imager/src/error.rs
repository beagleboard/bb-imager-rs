//! Command error type for this library

use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Error, Debug, Clone, Copy)]
pub enum Error {
    #[error("BeagleConnect Freedom Error: {0}")]
    BeagleConnectFreedomError(#[from] crate::bcf::Error),
    #[error("Download Error: {0}")]
    DownloadError(#[from] crate::download::Error)
}
