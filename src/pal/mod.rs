#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

use crate::DeviceDescriptor;

#[cfg(target_os = "windows")]
pub(crate) fn drive_list() -> anyhow::Result<Vec<DeviceDescriptor>> {
    windows::drive_list()
}

#[cfg(target_os = "linux")]
pub(crate) fn drive_list() -> anyhow::Result<Vec<DeviceDescriptor>> {
    linux::lsblk()
}

#[cfg(target_os = "macos")]
pub(crate) fn drive_list() -> anyhow::Result<Vec<DeviceDescriptor>> {
    macos::diskutil()
}
