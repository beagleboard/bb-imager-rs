use std::path::PathBuf;

pub mod bcf;
pub mod config;
pub mod sd;
pub mod download;

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
    Downloading,
    DownloadingProgress(f32),
    Finished(PathBuf),
}
