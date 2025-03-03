#[derive(Debug, Default, Clone)]
pub struct MountPoint {
    pub path: String,
    pub label: Option<String>,
    pub totalBytes: Option<u64>,
    pub availableBytes: Option<u64>,
}

impl MountPoint {
    pub fn new(path: impl ToString) -> Self {
        Self {
            path: path.to_string(),
            label: None,
            totalBytes: None,
            availableBytes: None,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(non_snake_case)]
pub struct DeviceDescriptor {
    pub enumerator: String,
    pub busType: Option<String>,
    pub busVersion: Option<String>,
    pub device: String,
    pub devicePath: Option<String>,
    pub raw: String,
    pub description: String,
    pub error: Option<String>,
    pub partitionTableType: Option<String>,
    pub size: u64,
    pub blockSize: u32,
    pub logicalBlockSize: u32,
    pub mountpoints: Vec<MountPoint>,
    pub mountpointLabels: Vec<String>,
    /// Device is read-only
    pub isReadOnly: bool,
    /// Device is a system drive
    pub isSystem: bool,
    /// Device is an SD-card
    pub isCard: bool,
    /// Connected via the Small Computer System Interface (SCSI)
    pub isSCSI: bool,
    /// Connected via Universal Serial Bus (USB)
    pub isUSB: bool,
    /// Device is a virtual storage device
    pub isVirtual: bool,
    /// Device is removable from the running system
    pub isRemovable: bool,
    /// Connected via the USB Attached SCSI (UAS)
    pub isUAS: Option<bool>,
}

impl Default for DeviceDescriptor {
    fn default() -> Self {
        Self {
            blockSize: 512,
            logicalBlockSize: 512,
            enumerator: Default::default(),
            busType: Default::default(),
            busVersion: Default::default(),
            device: Default::default(),
            devicePath: Default::default(),
            raw: Default::default(),
            description: Default::default(),
            error: Default::default(),
            partitionTableType: Default::default(),
            size: Default::default(),
            mountpoints: Default::default(),
            mountpointLabels: Default::default(),
            isReadOnly: Default::default(),
            isSystem: Default::default(),
            isCard: Default::default(),
            isSCSI: Default::default(),
            isUSB: Default::default(),
            isVirtual: Default::default(),
            isRemovable: Default::default(),
            isUAS: Default::default(),
        }
    }
}
