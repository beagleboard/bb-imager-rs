//! Stuff common to all the flashers

use std::{borrow::Cow, collections::HashSet};

use futures::channel::mpsc;
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
    VerifyingProgress(f32),
    Customizing,
}

/// A trait for modeling flashers. Also provides optional live status using channels.
pub trait BBFlasher {
    /// Start flashing. Generally, any image downloading should also be done as part of this
    /// function with the help of [ImageFile]
    ///
    /// [ImageFile]: crate::ImageFile
    fn flash(
        self,
        chan: Option<mpsc::Sender<DownloadFlashingStatus>>,
    ) -> impl Future<Output = anyhow::Result<()>>;
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

    /// A list of possible flasher targets
    fn destinations() -> impl Future<Output = HashSet<Self>>;

    /// Check if destination can be selected
    fn is_destination_selectable() -> bool;

    /// A sort of device ID (mostly a Path).
    fn identifier<'a>(&'a self) -> Cow<'a, str>;
}
