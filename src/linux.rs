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
        .join(" ")
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
            busType: Some(value.tran.as_deref().unwrap_or("UNKNOWN").to_uppercase()),
            device: value.name,
            raw: value.kname,
            isVirtual: is_virtual,
            isSCSI: is_scsi,
            isUSB: value.subsystems.contains("usb"),
            isReadOnly: value.ro,
            description,
            size: value.size,
            blockSize: value.phy_sec,
            logicalBlockSize: value.log_sec,
            isRemovable: is_removable,
            isSystem: is_system,
            partitionTableType: value.ptype,
            mountpoints: value.children.into_iter().map(Into::into).collect(),
            ..Default::default()
        }
    }
}

#[derive(Deserialize, Debug)]
struct Child {
    mountpoint: Option<String>,
    fssize: Option<u64>,
    fsavail: Option<u64>,
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
            totalBytes: value.fssize,
            availableBytes: value.fsavail,
            ..Default::default()
        }
    }
}

pub(crate) fn lsblk() -> anyhow::Result<Vec<DeviceDescriptor>> {
    let output = Command::new("lsblk")
        .args(["--bytes", "--all", "--json", "--paths", "--output-all"])
        .output()?;

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

    let res: Devices = serde_json::from_slice(&output.stdout).unwrap();

    Ok(res.blockdevices.into_iter().map(Into::into).collect())
}
