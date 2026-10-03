//! Lightweight cooperative cancellation.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// A cheaply cloneable token that can be polled for cancellation.
///
/// The token is cancelled when a [`DropGuard`] obtained from it is dropped. All clones share the
/// same state.
#[derive(Debug, Clone)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    /// Returns `true` if the token has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    /// Returns a guard that cancels this token when dropped.
    pub fn drop_guard(&self) -> DropGuard {
        DropGuard(self.0.clone())
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
}

/// Cancels the associated [`CancellationToken`] when dropped.
pub struct DropGuard(Arc<AtomicBool>);

impl Drop for DropGuard {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed)
    }
}
