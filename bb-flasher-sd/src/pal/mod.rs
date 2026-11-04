#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
pub(crate) use linux::{format, open};
#[cfg(target_os = "macos")]
pub(crate) use macos::{format, open};
#[cfg(windows)]
pub(crate) use windows::{format, open};
