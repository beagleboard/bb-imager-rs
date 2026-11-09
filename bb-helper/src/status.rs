//! Progress reported by the flashers.

use std::sync::mpsc::SyncSender;

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

/// Report `msg` on `chan` without blocking.
///
/// Progress is lossy by design: if the receiver is slow or gone, the message is dropped
/// rather than stalling the flash.
pub fn send(chan: Option<&SyncSender<Status>>, msg: Status) {
    if let Some(c) = chan {
        let _ = c.try_send(msg);
    }
}
