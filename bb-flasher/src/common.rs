//! Stuff common to all the flashers

use std::borrow::Cow;

/// Errors from flashing.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Failed to open or read the image.
    #[error("Failed to fetch image.")]
    Image {
        #[source]
        source: std::io::Error,
    },
    /// Flashing an SD card failed.
    #[cfg(feature = "sd")]
    #[error(transparent)]
    Sd(#[from] bb_flasher_sd::Error),
    /// Flashing BeagleConnect Freedom's CC1352P7 failed.
    #[cfg(feature = "bcf")]
    #[error(transparent)]
    Cc1352p7(#[from] bb_flasher_bcf::cc1352p7::Error),
    /// Flashing BeagleConnect Freedom's MSP430 failed.
    #[cfg(feature = "bcf_msp430")]
    #[error(transparent)]
    Msp430(#[from] bb_flasher_bcf::msp430::Error),
    /// Flashing an MSPM0 over UART or I2C failed.
    #[cfg(any(feature = "mspm0_uart", feature = "mspm0_i2c"))]
    #[error(transparent)]
    Mspm0(#[from] bb_flasher_mspm0::Error),
    /// Flashing PocketBeagle 2's MSPM0 failed.
    #[cfg(feature = "pb2_mspm0")]
    #[error(transparent)]
    Pb2Mspm0(#[from] bb_flasher_pb2_mspm0::Error),
    /// Flashing over USB DFU failed.
    #[cfg(feature = "dfu")]
    #[error(transparent)]
    Dfu(#[from] bb_flasher_dfu::Error),
    /// The target is not supported on this OS (e.g. MSPM0 over I2C off Linux).
    #[error("Unsupported OS.")]
    UnsupportedOs,
}

/// Result type for flashing.
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub use bb_helper::status::Status as DownloadFlashingStatus;

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
) -> Result<Vec<u8>> {
    use std::io::Read;

    let (mut img, size) = img().map_err(|source| Error::Image { source })?;

    // If size > usize::MAX, this function should never have been called in the first place. So
    // panic is fine
    let mut data = Vec::with_capacity(usize::try_from(size).expect("Image size too big"));
    img.read_to_end(&mut data)
        .map_err(|source| Error::Image { source })?;

    Ok(data)
}
