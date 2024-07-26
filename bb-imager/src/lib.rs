use std::path::PathBuf;

pub mod bcf;
pub mod config;
pub mod download;
pub mod error;
pub mod sd;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum FlashingStatus {
    Preparing,
    Flashing,
    FlashingProgress(f32),
    Verifying,
    VerifyingProgress(f32),
    Finished,
}

#[derive(Debug, PartialEq, Clone)]
pub enum DownloadStatus {
    DownloadingProgress(f32),
    VerifyingProgress(f32),
    Finished(PathBuf),
}
