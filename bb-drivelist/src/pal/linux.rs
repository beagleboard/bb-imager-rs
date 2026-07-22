use std::process::Command;

use crate::device::{DeviceDescriptor, MountPoint};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Devices {
    blockdevices: Vec<Device>,
}

#[derive(Deserialize, Debug)]
struct Device {
    size: Option<u64>,
    #[serde(default = "Device::name_default")]
    kname: String,
    #[serde(default = "Device::name_default")]
    name: String,
    tran: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    subsystems: Option<String>,
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
        self.subsystems.as_ref().is_some_and(|x| {
            x.contains("sata")
                || x.contains("scsi")
                || x.contains("ata")
                || x.contains("ide")
                || x.contains("pci")
        })
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
        self.subsystems
            .as_ref()
            .is_some_and(|x| !x.contains("block"))
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
            is_usb: value.subsystems.is_some_and(|x| x.contains("usb")),
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

pub(crate) fn lsblk() -> crate::Result<Vec<DeviceDescriptor>> {
    let output = Command::new("lsblk")
        .args(["--bytes", "--all", "--json", "--paths", "--output-all"])
        .output()
        .map_err(|e| crate::Error::LsblkExecuteError { source: Some(e) })?;

    if !output.status.success() {
        return Err(crate::Error::LsblkExecuteError { source: None });
    }

    let res: Devices = serde_json::from_slice(&output.stdout).unwrap();

    Ok(res.blockdevices.into_iter().map(Into::into).collect())
}

fn empty_string_as_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.filter(|s| !s.is_empty()))
}

#[cfg(test)]
mod tests {
    use crate::DeviceDescriptor;

    #[test]
    fn loop_dev() {
        let data = r#"
        {
            "blockdevices": [
                {
                    "name":"/dev/loop23", 
                    "kname":"/dev/loop23", 
                    "path":"/dev/loop23", 
                    "maj:min":"7:23", 
                    "fsavail":null, 
                    "fssize":null, 
                    "fstype":null, 
                    "fsused":null, 
                    "fsuse%":null, 
                    "mountpoint":null, 
                    "label":null, 
                    "uuid":null, 
                    "ptuuid":null, 
                    "pttype":null, 
                    "parttype":null, 
                    "partlabel":null, 
                    "partuuid":null, 
                    "partflags":null, 
                    "ra":128, 
                    "ro":false, 
                    "rm":false, 
                    "hotplug":false, 
                    "model":null, 
                    "serial":null, 
                    "size":null, 
                    "state":null, 
                    "owner":"root", 
                    "group":"disk", 
                    "mode":"brw-rw----", 
                    "alignment":0, 
                    "min-io":512, 
                    "opt-io":0, 
                    "phy-sec":512, 
                    "log-sec":512, 
                    "rota":false, 
                    "sched":"none", 
                    "rq-size":128, 
                    "type":"loop", 
                    "disc-aln":0, 
                    "disc-gran":4096, 
                    "disc-max":4294966784, 
                    "disc-zero":false, 
                    "wsame":0, 
                    "wwn":null, 
                    "rand":false, 
                    "pkname":null, 
                    "hctl":null, 
                    "tran":null, 
                    "subsystems":"block", 
                    "rev":null, 
                    "vendor":null, 
                    "zoned":"none"
                }
            ]
        }"#;

        let res: super::Devices = serde_json::from_str(data).unwrap();
        let _: Vec<DeviceDescriptor> = res.blockdevices.into_iter().map(Into::into).collect();
    }

    /// Parse a `blockdevices` JSON array through the same path `lsblk()` uses
    /// (`Devices` -> `DeviceDescriptor`), so the classification logic can be
    /// exercised without a real `lsblk` binary or block devices.
    fn descriptors(blockdevices: &str) -> Vec<DeviceDescriptor> {
        let data = format!(r#"{{"blockdevices":{blockdevices}}}"#);
        let res: super::Devices = serde_json::from_str(&data).unwrap();
        res.blockdevices.into_iter().map(Into::into).collect()
    }

    /// A removable USB device: `rm` + a `usb`/`scsi` subsystem string. It should
    /// be classified removable (not system), the bus type upper-cased, and the
    /// description built from label+vendor+model with empty parts dropped.
    #[test]
    fn usb_removable_disk_classification() {
        let d = &descriptors(
            r#"[{
                "name":"/dev/sda","kname":"/dev/sda",
                "size":32000000000,"tran":"usb",
                "subsystems":"block:scsi:usb:pci","ro":false,
                "phy-sec":512,"log-sec":512,"rm":true,"hotplug":false,
                "ptype":"gpt","label":"BOOT","vendor":"Kingston","model":"DataTraveler"
            }]"#,
        )[0];

        assert_eq!(d.enumerator, "lsblk:json");
        assert_eq!(d.device, "/dev/sda");
        assert_eq!(d.raw, "/dev/sda");
        assert_eq!(d.bus_type.as_deref(), Some("USB"));
        assert!(d.is_usb);
        assert!(d.is_scsi);
        assert!(!d.is_virtual);
        assert!(d.is_removable);
        assert!(!d.is_system);
        assert!(!d.is_readonly);
        assert_eq!(d.size, Some(32000000000));
        assert_eq!(d.block_size, 512);
        assert_eq!(d.logical_block_size, 512);
        assert_eq!(d.partition_table_type.as_deref(), Some("gpt"));
        assert_eq!(d.description, "BOOTKingstonDataTraveler");
    }

    /// An internal NVMe disk: no `usb`, not `rm`/`hotplug`, and `block` present.
    /// It should be a non-removable system drive, `is_scsi` via the `pci`
    /// subsystem, description built from `model` alone.
    #[test]
    fn internal_disk_is_system_not_removable() {
        let d = &descriptors(
            r#"[{
                "name":"/dev/nvme0n1","kname":"/dev/nvme0n1",
                "size":512000000000,"tran":"nvme",
                "subsystems":"block:nvme:pci","ro":false,
                "phy-sec":512,"log-sec":4096,"rm":false,"hotplug":false,
                "ptype":null,"label":null,"vendor":null,"model":"Samsung SSD"
            }]"#,
        )[0];

        assert_eq!(d.bus_type.as_deref(), Some("NVME"));
        assert!(!d.is_usb);
        assert!(d.is_scsi);
        assert!(!d.is_virtual);
        assert!(!d.is_removable);
        assert!(d.is_system);
        assert_eq!(d.logical_block_size, 4096);
        assert_eq!(d.description, "Samsung SSD");
        assert_eq!(d.partition_table_type, None);
    }

    /// A device whose `subsystems` string lacks `block` is virtual, which forces
    /// removable=true and system=false. A missing `tran` yields the "UNKNOWN"
    /// bus type, and `ro` propagates to `is_readonly`.
    #[test]
    fn virtual_device_without_block_subsystem() {
        let d = &descriptors(
            r#"[{
                "name":"/dev/dm-0","kname":"/dev/dm-0",
                "size":null,"tran":null,
                "subsystems":"nvme:pci",
                "ro":true,
                "phy-sec":512,"log-sec":512,"rm":false,"hotplug":false,
                "ptype":null,"label":null,"vendor":null,"model":null
            }]"#,
        )[0];

        assert!(d.is_virtual);
        assert!(d.is_removable);
        assert!(!d.is_system);
        assert_eq!(d.bus_type.as_deref(), Some("UNKNOWN"));
        assert!(d.is_readonly);
        assert!(!d.is_usb);
        assert_eq!(d.description, "");
    }

    /// `hotplug` alone (without `rm`) must still mark the device removable.
    #[test]
    fn hotplug_alone_marks_removable() {
        let d = &descriptors(
            r#"[{
                "name":"/dev/sdb","kname":"/dev/sdb",
                "size":8000000000,"tran":"usb",
                "subsystems":"block:usb","ro":false,
                "phy-sec":512,"log-sec":512,"rm":false,"hotplug":true,
                "ptype":null,"label":null,"vendor":null,"model":null
            }]"#,
        )[0];

        assert!(d.is_removable);
        assert!(!d.is_system);
        assert!(d.is_usb);
        assert!(!d.is_virtual);
    }

    /// Children map to mountpoints: `fssize`/`fsavail` accept either JSON strings
    /// or numbers (the `FsSize` untagged enum), the mount label falls back to
    /// `partlabel` when `label` is null, and a null `mountpoint` becomes "".
    #[test]
    fn children_map_to_mountpoints_with_fssize_variants() {
        let d = &descriptors(
            r#"[{
                "name":"/dev/sdc","kname":"/dev/sdc",
                "size":16000000000,"tran":"usb",
                "subsystems":"block:usb","ro":false,
                "phy-sec":512,"log-sec":512,"rm":true,"hotplug":false,
                "ptype":null,"label":null,"vendor":null,"model":null,
                "children":[
                    {"mountpoint":"/boot","fssize":"1048576","fsavail":524288,"label":null,"partlabel":"BOOTFS"},
                    {"mountpoint":null,"fssize":null,"fsavail":null,"label":"ROOT","partlabel":"rootfs"}
                ]
            }]"#,
        )[0];

        assert_eq!(d.mountpoints.len(), 2);

        // fssize given as a JSON string, fsavail as a number.
        assert_eq!(d.mountpoints[0].path, "/boot");
        assert_eq!(d.mountpoints[0].total_bytes, Some(1048576));
        assert_eq!(d.mountpoints[0].available_bytes, Some(524288));
        // label is null -> falls back to partlabel.
        assert_eq!(d.mountpoints[0].label.as_deref(), Some("BOOTFS"));

        // null mountpoint -> empty path; label present -> used as-is.
        assert_eq!(d.mountpoints[1].path, "");
        assert_eq!(d.mountpoints[1].total_bytes, None);
        assert_eq!(d.mountpoints[1].label.as_deref(), Some("ROOT"));
    }

    /// Missing `name`/`kname` fall back to the `NO_NAME` default.
    #[test]
    fn missing_name_uses_default() {
        let d = &descriptors(
            r#"[{
                "size":null,"tran":null,
                "subsystems":"block","ro":false,
                "phy-sec":512,"log-sec":512,"rm":false,"hotplug":false,
                "ptype":null,"label":null,"vendor":null,"model":null
            }]"#,
        )[0];

        assert_eq!(d.device, "NO_NAME");
        assert_eq!(d.raw, "NO_NAME");
    }

    /// `subsystems` can be null in `lsblk` output. The device must still parse
    /// (it used to fail deserialization when the field was a required `String`),
    /// and every subsystems-derived flag falls to false: `is_scsi`, `is_usb`,
    /// and `is_virtual`. Note `is_usb` is derived from `subsystems`, not `tran`,
    /// so it stays false even though `tran` is "usb" and the bus type is "USB".
    /// With no `rm`/`hotplug` and not virtual, the device is a non-removable
    /// system disk.
    #[test]
    fn null_subsystems_parses_and_classifies() {
        let d = &descriptors(
            r#"[{
                "name":"/dev/sdd","kname":"/dev/sdd",
                "size":64000000000,"tran":"usb",
                "subsystems":null,"ro":false,
                "phy-sec":512,"log-sec":512,"rm":false,"hotplug":false,
                "ptype":null,"label":null,"vendor":null,"model":"Generic"
            }]"#,
        )[0];

        assert!(!d.is_scsi);
        assert!(!d.is_usb);
        assert!(!d.is_virtual);
        assert!(!d.is_removable);
        assert!(d.is_system);
        // Non-subsystems fields are unaffected: bus type still comes from `tran`.
        assert_eq!(d.bus_type.as_deref(), Some("USB"));
        assert_eq!(d.description, "Generic");
    }

    /// The `subsystems` key omitted entirely (not just null) must also parse via
    /// serde's `Option` default, yielding the same all-false classification.
    #[test]
    fn omitted_subsystems_key_parses() {
        let d = &descriptors(
            r#"[{
                "name":"/dev/sde","kname":"/dev/sde",
                "size":null,"tran":null,"ro":false,
                "phy-sec":512,"log-sec":512,"rm":false,"hotplug":false,
                "ptype":null,"label":null,"vendor":null,"model":null
            }]"#,
        )[0];

        assert!(!d.is_scsi);
        assert!(!d.is_usb);
        assert!(!d.is_virtual);
        assert_eq!(d.bus_type.as_deref(), Some("UNKNOWN"));
    }

    /// Guard against a regression where a virtual device is detected by the
    /// *absence* of "block" in `subsystems`: a null `subsystems` is NOT the same
    /// as a subsystems string missing "block". `is_some_and` returns false for
    /// None, so a null-subsystems device is non-virtual (contrast with the
    /// existing empty-string case, which is virtual).
    #[test]
    fn null_subsystems_is_not_virtual() {
        let d = &descriptors(
            r#"[{
                "name":"/dev/dm-1","kname":"/dev/dm-1",
                "size":null,"tran":null,"subsystems":null,"ro":false,
                "phy-sec":512,"log-sec":512,"rm":false,"hotplug":false,
                "ptype":null,"label":null,"vendor":null,"model":null
            }]"#,
        )[0];

        assert!(!d.is_virtual);
        assert!(!d.is_removable);
        assert!(d.is_system);
    }

    /// An empty `subsystems` string is normalized to `None`, so it classifies
    /// identically to null/missing: not virtual (contrast the old behavior where
    /// `""` was treated as virtual because it lacks "block").
    #[test]
    fn empty_subsystems_normalized_to_none() {
        let d = &descriptors(
            r#"[{
                "name":"/dev/dm-2","kname":"/dev/dm-2",
                "size":null,"tran":null,"subsystems":"","ro":false,
                "phy-sec":512,"log-sec":512,"rm":false,"hotplug":false,
                "ptype":null,"label":null,"vendor":null,"model":null
            }]"#,
        )[0];

        assert!(!d.is_virtual);
        assert!(!d.is_scsi);
        assert!(!d.is_usb);
    }
}
