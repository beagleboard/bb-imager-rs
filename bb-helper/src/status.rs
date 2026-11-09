//! Progress reported by the flashers.

/// Flashing status.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Status {
    /// Preparing the destination for flashing.
    Preparing,
    /// The image is being written. Progress is in `0.0..=1.0`.
    Writing(f32),
    /// Verifying the written image.
    Verifying,
    /// Applying post-install customization.
    Customizing,
}
