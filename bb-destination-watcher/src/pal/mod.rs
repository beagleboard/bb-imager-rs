//! Platform backends.
//!
//! Each provides the same private contract, which is the entire per-OS surface:
//!
//! ```ignore
//! pub(crate) struct Watcher { /* .. */ }
//! impl Watcher { pub(crate) async fn start() -> Result<Self, crate::Error>; }
//! impl futures_core::Stream for Watcher { type Item = (); }
//! impl Drop for Watcher { /* unregister */ }
//! ```
//!
//! `Watcher` must be `Unpin`.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub(crate) use linux::Watcher;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub(crate) use macos::Watcher;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use windows::Watcher;

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod unsupported;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
pub(crate) use unsupported::Watcher;
