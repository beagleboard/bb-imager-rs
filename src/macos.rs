use std::process::Command;

use serde::Deserialize;

use crate::device::{DeviceDescriptor, MountPoint};

#[derive(Deserialize, Debug)]
struct Disks {
    #[serde(rename = "AllDisksAndPartitions")]
    all_disks_and_partitions: Vec<Disk>,
}

#[derive(Deserialize, Debug)]
struct Disk {
    #[serde(rename = "DeviceIdentifier")]
    device_identifier: String,
    #[serde(rename = "OSInternal")]
    os_internal: bool,
    #[serde(rename = "Size")]
    size: u64,
    #[serde(rename = "Content")]
    content: String,
    #[serde(rename = "Partitions")]
    partitions: Vec<Partition>,
}

#[derive(Deserialize, Debug)]
struct Partition {
    #[serde(rename = "MountPoint")]
    mount_point: Option<String>,
    #[serde(rename = "Content")]
    content: String,
    #[serde(rename = "Size")]
    size: u64,
}

impl From<Disk> for DeviceDescriptor {
    fn from(value: Disk) -> Self {
        DeviceDescriptor {
            enumerator: "diskutil".to_string(),
            description: value.content,
            size: value.size,
            mountpoints: value.partitions.into_iter().map(MountPoint::from).collect(),
            device: format!("/dev/{}", value.device_identifier),
            raw: format!("/dev/r{}", value.device_identifier),
            isSystem: value.os_internal,
            isRemovable: !value.os_internal,
            ..Default::default()
        }
    }
}

impl From<Partition> for MountPoint {
    fn from(value: Partition) -> Self {
        MountPoint {
            path: value.mount_point.unwrap_or_default(),
            label: Some(value.content),
            totalBytes: Some(value.size),
            availableBytes: None,
        }
    }
}

pub(crate) fn diskutil() -> anyhow::Result<Vec<DeviceDescriptor>> {
    let output = Command::new("diskutil").args(["list", "-plist"]).output()?;

    if let Some(code) = output.status.code() {
        if code != 0 {
            return Err(anyhow::Error::msg(format!("lsblk ExitCode: {}", code)));
        }
    }

    if output.stderr.len() > 0 {
        return Err(anyhow::Error::msg(format!(
            "lsblk stderr: {}",
            std::str::from_utf8(&output.stderr).unwrap()
        )));
    }

    let parsed: Disks = plist::from_bytes(&output.stdout).unwrap();

    Ok(parsed
        .all_disks_and_partitions
        .into_iter()
        .map(DeviceDescriptor::from)
        .collect())
}
