//! Command error type for this library

use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Io Error: {0}")]
    IoError(#[from] std::io::Error),
}
