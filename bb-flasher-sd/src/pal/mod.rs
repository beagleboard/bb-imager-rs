#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
pub(crate) use linux::{open, format};
#[cfg(target_os = "macos")]
pub(crate) use macos::{open, format};
#[cfg(windows)]
pub(crate) use windows::{open, format};
