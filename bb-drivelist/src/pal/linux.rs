use std::process::Command;

use crate::device::{DeviceDescriptor, MountPoint};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Devices {
    blockdevices: Vec<Device>,
}

#[derive(Deserialize, Debug)]
struct Device {
    size: u64,
    #[serde(default = "Device::name_default")]
    kname: String,
    #[serde(default = "Device::name_default")]
    name: String,
    tran: Option<String>,
    subsystems: String,
    ro: bool,
    #[serde(rename = "phy-sec")]
    phy_sec: u32,
    #[serde(rename = "log-sec")]
    log_sec: u32,
    rm: bool,
    ptype: Option<String>,
    #[serde(default)]
    children: Vec<Child>,
    label: Option<String>,
    vendor: Option<String>,
    model: Option<String>,
    hotplug: bool,
}

impl Device {
    fn name_default() -> String {
        "NO_NAME".to_string()
    }

    fn is_scsi(&self) -> bool {
        self.subsystems.contains("sata")
            || self.subsystems.contains("scsi")
            || self.subsystems.contains("ata")
            || self.subsystems.contains("ide")
            || self.subsystems.contains("pci")
    }

    fn description(&self) -> String {
        [
            self.label.as_deref().unwrap_or_default(),
            self.vendor.as_deref().unwrap_or_default(),
            self.model.as_deref().unwrap_or_default(),
        ]
        .into_iter()
        .filter(|x| !x.is_empty())
        .fold(String::new(), |mut acc, x| {
            acc.push_str(x);
            acc
        })
    }

    fn is_virtual(&self) -> bool {
        !self.subsystems.contains("block")
    }

    fn is_removable(&self) -> bool {
        self.rm || self.hotplug || self.is_virtual()
    }

    fn is_system(&self) -> bool {
        !(self.is_removable() || self.is_virtual())
    }
}

impl From<Device> for DeviceDescriptor {
    fn from(value: Device) -> Self {
        let is_scsi = value.is_scsi();
        let description = value.description();
        let is_virtual = value.is_virtual();
        let is_removable = value.is_removable();
        let is_system = value.is_system();

        Self {
            enumerator: "lsblk:json".to_string(),
            bus_type: Some(value.tran.as_deref().unwrap_or("UNKNOWN").to_uppercase()),
            device: value.name,
            raw: value.kname,
            is_virtual,
            is_scsi,
            is_usb: value.subsystems.contains("usb"),
            is_readonly: value.ro,
            description,
            size: value.size,
            block_size: value.phy_sec,
            logical_block_size: value.log_sec,
            is_removable,
            is_system,
            partition_table_type: value.ptype,
            mountpoints: value.children.into_iter().map(Into::into).collect(),
            ..Default::default()
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
/// Sometimes fssize and fsavail are strings. So need to handle that.
enum FsSize {
    String(String),
    U64(u64),
}

impl From<FsSize> for u64 {
    fn from(value: FsSize) -> Self {
        match value {
            FsSize::String(x) => x.parse().unwrap(),
            FsSize::U64(x) => x,
        }
    }
}

#[derive(Deserialize, Debug)]
struct Child {
    mountpoint: Option<String>,
    fssize: Option<FsSize>,
    fsavail: Option<FsSize>,
    label: Option<String>,
    partlabel: Option<String>,
}

impl From<Child> for MountPoint {
    fn from(value: Child) -> Self {
        Self {
            path: value.mountpoint.unwrap_or_default(),
            label: if value.label.is_some() {
                value.label
            } else {
                value.partlabel
            },
            total_bytes: value.fssize.map(Into::into),
            available_bytes: value.fsavail.map(Into::into),
        }
    }
}

pub(crate) fn lsblk() -> anyhow::Result<Vec<DeviceDescriptor>> {
    let output = Command::new("lsblk")
        .args(["--bytes", "--all", "--json", "--paths", "--output-all"])
        .output()?;

    if !output.status.success() {
        return Err(anyhow::Error::msg("lsblk fail"));
    }

    let res: Devices = serde_json::from_slice(&output.stdout).unwrap();

    Ok(res.blockdevices.into_iter().map(Into::into).collect())
}
