//! Stuff common to all the flashers

use std::{borrow::Cow, collections::HashSet, io::Read};

use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum FlasherError {
    #[error("Failed to fetch image.")]
    ImageResolvingError {
        #[source]
        source: std::io::Error,
    },
}

/// Enum to denote the Flashing progress.
///
/// The progress is denoted by [f32] between 0 and 1
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DownloadFlashingStatus {
    Preparing,
    DownloadingProgress(f32),
    FlashingProgress(f32),
    Verifying,
    Customizing,
}

/// A trait for modeling flasher targets.
///
/// Some flashers have a single target (for example a subprocessor in SBC).
pub trait BBFlasherTarget
where
    Self: Sized,
{
    /// File types (extensions) supported by the flasher. Can be used for filtering local files in
    /// applications
    const FILE_TYPES: &[&str];
    const IS_DESTINATION_SELECTABLE: bool = true;

    /// A list of possible flasher targets
    fn destinations(filter: bool) -> HashSet<Self>;

    /// A sort of device ID (mostly a Path).
    fn identifier<'a>(&'a self) -> Cow<'a, str>;
}

// Should only be used when image is expected to rather small and can fit in heap.
pub(crate) fn resolve_img(
    img: impl FnOnce() -> std::io::Result<(crate::OsImage, u64)>,
) -> Result<Vec<u8>, FlasherError> {
    let (mut img, _) = img().map_err(|source| FlasherError::ImageResolvingError { source })?;

    let mut data = Vec::new();
    img.read_to_end(&mut data)
        .map_err(|source| FlasherError::ImageResolvingError { source })?;

    Ok(data)
}
