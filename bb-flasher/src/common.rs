//! Stuff common to all the flashers

use std::borrow::Cow;

#[cfg(any(
    feature = "bcf",
    feature = "bcf_msp430",
    feature = "pb2_mspm0",
    feature = "mspm0_uart",
    feature = "mspm0_i2c"
))]
#[derive(thiserror::Error, Debug)]
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
    /// Preparing for flashing.
    Preparing,
    /// Downloading the image.
    DownloadingProgress(f32),
    /// Writing the image to the target.
    FlashingProgress(f32),
    /// Verifying the written image.
    Verifying,
    /// Applying post-install customization.
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
    /// `false` if the flasher has a single fixed target, so applications need not ask the user to
    /// select one.
    const IS_DESTINATION_SELECTABLE: bool = true;

    /// A list of possible flasher targets
    fn destinations(filter: bool) -> impl Iterator<Item = Self>;

    /// A sort of device ID (mostly a Path).
    fn identifier<'a>(&'a self) -> Cow<'a, str>;
}

// Should only be used when image is expected to rather small and can fit in heap.
#[cfg(any(
    feature = "bcf",
    feature = "bcf_msp430",
    feature = "pb2_mspm0",
    feature = "mspm0_uart",
    feature = "mspm0_i2c"
))]
pub(crate) fn resolve_img(
    img: impl FnOnce() -> std::io::Result<(crate::img::OsImage, u64)>,
) -> Result<Vec<u8>, FlasherError> {
    let (mut img, size) = img().map_err(|source| FlasherError::ImageResolvingError { source })?;

    // If size > usize::MAX, this function should never have been called in the first place. So
    // panic is fine
    let mut data = Vec::with_capacity(usize::try_from(size).expect("Image size too big"));
    std::io::Read::read_to_end(&mut img, &mut data)
        .map_err(|source| FlasherError::ImageResolvingError { source })?;

    Ok(data)
}
