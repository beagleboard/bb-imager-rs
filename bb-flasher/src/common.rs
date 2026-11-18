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
    /// The image is too large to be firmware for this flasher (e.g. an SD card image picked by
    /// mistake).
    #[error("Image is too large for this flasher: {size} bytes, at most {max} bytes supported.")]
    ImageTooLarge {
        /// Image size in bytes (reported or read so far).
        size: u64,
        /// Largest supported size in bytes.
        max: u64,
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

/// Largest image [`resolve_img`] reads into memory. Microcontroller firmware is at most a few MiB
/// even as text (CC1352P7: 704 KiB flash, ~2 MiB as Intel HEX).
#[cfg(any(
    feature = "bcf",
    feature = "bcf_msp430",
    feature = "pb2_mspm0",
    feature = "mspm0_uart",
    feature = "mspm0_i2c"
))]
const MAX_IMG_SIZE: u64 = 16 * 1024 * 1024;

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

    let (img, size) = img().map_err(|source| Error::Image { source })?;
    let too_large = |size| Error::ImageTooLarge {
        size,
        max: MAX_IMG_SIZE,
    };
    if size > MAX_IMG_SIZE {
        return Err(too_large(size));
    }

    let mut data = Vec::with_capacity(usize::try_from(size).expect("bounded by MAX_IMG_SIZE"));
    // The size comes from image metadata (xz index, remote config), so cap the read as well.
    img.take(MAX_IMG_SIZE + 1)
        .read_to_end(&mut data)
        .map_err(|source| Error::Image { source })?;
    if data.len() as u64 > MAX_IMG_SIZE {
        return Err(too_large(data.len() as u64));
    }

    Ok(data)
}

#[cfg(all(
    test,
    any(
        feature = "bcf",
        feature = "bcf_msp430",
        feature = "pb2_mspm0",
        feature = "mspm0_uart",
        feature = "mspm0_i2c"
    )
))]
mod tests {
    use super::*;
    use std::io::Write;

    /// xz file of `len` zero bytes; compresses to a few KiB, so the test stays cheap.
    fn zeros_xz(len: usize) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(&liblzma::encode_all(&vec![0u8; len][..], 1).unwrap())
            .unwrap();
        f
    }

    #[test]
    fn resolve_img_rejects_oversized_reported_size() {
        let f = zeros_xz(MAX_IMG_SIZE as usize + 1);
        let err = resolve_img(|| {
            let img = crate::img::OsImage::from_path(f.path())?;
            let size = img.size();
            Ok((img, size))
        })
        .unwrap_err();
        assert!(matches!(err, Error::ImageTooLarge { size, .. } if size == MAX_IMG_SIZE + 1));
    }

    #[test]
    fn resolve_img_caps_read_when_size_lies() {
        let f = zeros_xz(MAX_IMG_SIZE as usize + 1);
        // Metadata claims a tiny image, as a wrong remote `extract_size` would.
        let err = resolve_img(|| Ok((crate::img::OsImage::from_path(f.path())?, 10))).unwrap_err();
        assert!(matches!(err, Error::ImageTooLarge { .. }));
    }

    #[test]
    fn resolve_img_reads_small_image() {
        let f = zeros_xz(1024);
        let data = resolve_img(|| Ok((crate::img::OsImage::from_path(f.path())?, 1024))).unwrap();
        assert_eq!(data.len(), 1024);
    }
}
