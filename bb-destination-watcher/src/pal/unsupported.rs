//! Fallback for platforms with no backend.
//!
//! This exists so "no watcher available" is an ordinary runtime outcome the
//! caller already handles rather than a build failure, and so `pal/mod.rs`
//! needs a single `cfg` block instead of one per item.

use std::pin::Pin;
use std::task::{Context, Poll};

use crate::Error;

pub(crate) struct Watcher(std::convert::Infallible);

impl Watcher {
    pub(crate) async fn start() -> Result<Self, Error> {
        Err(Error::Unsupported)
    }
}

impl futures_core::Stream for Watcher {
    type Item = ();

    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Unconstructible, so this is unreachable.
        match self.0 {}
    }
}
