//! Notifies when the set of attached storage destinations may have changed.
//!
//! Enumerating destinations is expensive on some platforms — on Linux it forks
//! `lsblk`, which costs milliseconds and thousands of syscalls. Polling it on a
//! timer means paying that continuously for as long as a destination list is on
//! screen. This crate lets a caller wait for the OS to say something changed and
//! re-enumerate only then.
//!
//! # What this is not
//!
//! [`Watcher`] reports *that* something changed, never *what*. Hotplug
//! notifications differ enough between Linux, macOS and Windows that a shared
//! delta type would be mostly lies, and consumers need a full list to render
//! anyway. So the contract is deliberately minimal: on each item, re-enumerate.
//!
//! # Usage
//!
//! [`Watcher`] emits nothing on a schedule of its own, and events can be missed
//! across suspend/resume or a dropped connection. Pair it with a slow fallback
//! timer, and fall back to polling if construction fails:
//!
//! ```no_run
//! # async fn example() {
//! use futures_util::StreamExt;
//!
//! match bb_destination_watcher::Watcher::new().await {
//!     Ok(mut watcher) => {
//!         while watcher.next().await.is_some() {
//!             // re-enumerate destinations
//!         }
//!     }
//!     Err(e) => {
//!         tracing::info!("no destination watcher ({e}), falling back to polling");
//!     }
//! }
//! # }
//! ```

use std::pin::Pin;
use std::task::{Context, Poll};

mod pal;

/// Reasons a [`Watcher`] could not be started.
///
/// Callers are not expected to distinguish these beyond logging: every variant
/// means the same thing operationally, which is to fall back to polling.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// This platform has no watcher implementation.
    #[error("destination watching is unsupported on this platform")]
    Unsupported,
    /// The platform API exists but could not be started.
    #[error("failed to start destination watcher: {0}")]
    Backend(String),
}

/// A stream of hints that the set of attached storage destinations may have
/// changed.
///
/// Each item means "re-enumerate"; there is no payload. See the [crate docs](crate)
/// for why, and for the fallback timer this should be paired with.
///
/// Stops watching when dropped.
pub struct Watcher(pal::Watcher);

impl Watcher {
    /// Start watching for storage device changes.
    ///
    /// Any error means the caller should poll instead.
    pub async fn new() -> Result<Self, Error> {
        pal::Watcher::start().await.map(Self)
    }
}

impl futures_core::Stream for Watcher {
    type Item = ();

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Every platform backend is `Unpin`: they hold either a boxed stream or
        // an mpsc receiver.
        Pin::new(&mut self.get_mut().0).poll_next(cx)
    }
}

impl std::fmt::Debug for Watcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Watcher").finish_non_exhaustive()
    }
}
