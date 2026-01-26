use core::ffi::c_void;
use std::collections::HashMap;
use std::ffi::{c_char, CStr};
use std::ptr::NonNull;

use crate::device::DeviceDescriptor;
use crate::MountPoint;
use objc2::runtime::AnyObject;
use objc2::{rc::Retained, sel};
use objc2_core_foundation::{
    CFBoolean, CFDictionary, CFNumber, CFRetained, CFRunLoop, CFString, CFType,
    kCFAllocatorDefault, kCFRunLoopDefaultMode,
};
use objc2_disk_arbitration::{
    DADisk, DARegisterDiskAppearedCallback, DASession, DAUnregisterCallback,
    kDADiskDescriptionBusPathKey, kDADiskDescriptionDeviceInternalKey,
    kDADiskDescriptionDeviceProtocolKey, kDADiskDescriptionMediaBlockSizeKey,
    kDADiskDescriptionMediaContentKey, kDADiskDescriptionMediaEjectableKey,
    kDADiskDescriptionMediaIconKey, kDADiskDescriptionMediaNameKey,
    kDADiskDescriptionMediaRemovableKey, kDADiskDescriptionMediaSizeKey,
    kDADiskDescriptionMediaWritableKey,
};
use objc2_foundation::{
    NSArray, NSFileManager, NSMutableArray, NSNumber, NSString, NSURLVolumeLocalizedNameKey,
    NSURLVolumeNameKey, NSVolumeEnumerationOptions,
};

// UTILS

// Cached NSStrings for constant comparisons (avoids repeated allocations).
// Use thread-local storage since NSString/Retained are not Sync.
thread_local! {
    static SCSI_SATA: Retained<NSString> = NSString::from_str("SATA");
    static SCSI_SCSI: Retained<NSString> = NSString::from_str("SCSI");
    static SCSI_ATA: Retained<NSString> = NSString::from_str("ATA");
    static SCSI_IDE: Retained<NSString> = NSString::from_str("IDE");
    static SCSI_PCI: Retained<NSString> = NSString::from_str("PCI");

    static GUID_PARTITION_SCHEME: Retained<NSString> =
        NSString::from_str("GUID_partition_scheme");
    static FDISK_PARTITION_SCHEME: Retained<NSString> =
        NSString::from_str("FDisk_partition_scheme");
    static VIRTUAL_INTERFACE: Retained<NSString> = NSString::from_str("Virtual Interface");
    static IO_BUNDLE_RESOURCE_FILE: CFRetained<CFString> =
        CFString::from_str("IOBundleResourceFile");
}

/// Check if a given NSString matches any of the known SCSI type names.
/// Uses cached NSStrings to avoid String allocation per call.
fn scsi_type_matches(s: &NSString) -> bool {
    SCSI_SATA.with(|sata| s.isEqualToString(sata))
        || SCSI_SCSI.with(|scsi| s.isEqualToString(scsi))
        || SCSI_ATA.with(|ata| s.isEqualToString(ata))
        || SCSI_IDE.with(|ide| s.isEqualToString(ide))
        || SCSI_PCI.with(|pci| s.isEqualToString(pci))
}

/// Check if a BSD name matches the partition pattern (e.g., "disk0s1", "disk1s2").
/// Replaces NSPredicate regex with pure Rust for better performance.
fn is_partition_name(name: &NSString) -> bool {
    let s = name.to_string();
    let Some(rest) = s.strip_prefix("disk") else {
        return false;
    };
    let Some(s_pos) = rest.find('s') else {
        return false;
    };
    // Must have digits before 's' and digits after 's'
    let before_s = &rest[..s_pos];
    let after_s = &rest[s_pos + 1..];
    !before_s.is_empty()
        && before_s.chars().all(|c| c.is_ascii_digit())
        && !after_s.is_empty()
        && after_s.chars().all(|c| c.is_ascii_digit())
}

/// Extension trait for CFDictionary to get typed values
trait CFDictionaryExt {
    fn get_cfdict(&self, key: &CFString) -> Option<CFRetained<CFDictionary>>;
    fn get_cfstring(&self, key: &CFString) -> Option<CFRetained<CFString>>;
    fn get_number(&self, key: &CFString) -> Option<Retained<NSNumber>>;
    fn get_string(&self, key: &CFString) -> Option<Retained<NSString>>;
}

impl CFDictionaryExt for CFDictionary {
    fn get_cfdict(&self, key: &CFString) -> Option<CFRetained<CFDictionary>> {
        unsafe {
            let value = self.value(key as *const _ as *const c_void);
            if value.is_null() {
                return None;
            }
            let cf = (value as *const CFType).as_ref()?;
            let dict = cf.downcast_ref::<CFDictionary>()?;
            Some(CFRetained::retain(NonNull::from(dict)))
        }
    }

    fn get_cfstring(&self, key: &CFString) -> Option<CFRetained<CFString>> {
        unsafe {
            let value = self.value(key as *const _ as *const c_void);
            if value.is_null() {
                return None;
            }
            let cf = (value as *const CFType).as_ref()?;
            let s = cf.downcast_ref::<CFString>()?;
            Some(CFRetained::retain(NonNull::from(s)))
        }
    }

    fn get_number(&self, key: &CFString) -> Option<Retained<NSNumber>> {
        unsafe {
            let value = self.value(key as *const _ as *const c_void);
            if value.is_null() {
                None
            } else {
                let cf = (value as *const CFType).as_ref()?;
                if cf.downcast_ref::<CFNumber>().is_none()
                    && cf.downcast_ref::<CFBoolean>().is_none()
                {
                    return None;
                }
                Some(Retained::retain(value as *mut NSNumber)?)
            }
        }
    }

    fn get_string(&self, key: &CFString) -> Option<Retained<NSString>> {
        unsafe {
            let value = self.value(key as *const _ as *const c_void);
            if value.is_null() {
                None
            } else {
                let cf = (value as *const CFType).as_ref()?;
                if cf.downcast_ref::<CFString>().is_none() {
                    return None;
                }
                Some(Retained::retain(value as *mut NSString)?)
            }
        }
    }
}

// Extension trait for *const c_char to convert to a Rust String.
// Provides to_string() which returns Option<String> (None for null pointers).
trait CCharPtrExt {
    fn to_string(self) -> Option<String>;
}

impl CCharPtrExt for *const c_char {
    fn to_string(self) -> Option<String> {
        if self.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(self).to_string_lossy().into_owned() })
        }
    }
}

// DISKLIST

unsafe extern "C-unwind" fn append_disk(disk: NonNull<DADisk>, context: *mut c_void) {
    if context.is_null() {
        return;
    }

    let disks = context as *const Retained<NSMutableArray<NSString>>;
    let disks = unsafe { &*disks };

    let bsd_name = unsafe { disk.as_ref().bsd_name() };
    let Some(bsd_name_str) = bsd_name.to_string() else {
        return;
    };

    let bsd_name_nsstring = NSString::from_str(&bsd_name_str);

    if !disks.containsObject(&bsd_name_nsstring) {
        disks.addObject(&bsd_name_nsstring);
    }
}

struct DiskList {
    disks: Retained<NSMutableArray<NSString>>,
}

struct DiskAppearedCallbackGuard<'a> {
    session: &'a DASession,
    context: *mut c_void,
    callback: NonNull<c_void>,
}

impl<'a> DiskAppearedCallbackGuard<'a> {
    fn register(session: &'a DASession, context: *mut c_void) -> Self {
        let callback = NonNull::new(append_disk as *const () as *mut c_void)
            .expect("append_disk callback pointer is non-null");
        unsafe { DARegisterDiskAppearedCallback(session, None, Some(append_disk), context) };
        Self {
            session,
            context,
            callback,
        }
    }
}

impl Drop for DiskAppearedCallbackGuard<'_> {
    fn drop(&mut self) {
        unsafe { DAUnregisterCallback(self.session, self.callback, self.context) };
    }
}

impl DiskList {
    fn new() -> Self {
        let disks: Retained<NSMutableArray<NSString>> = NSMutableArray::new();
        let mut disk_list = Self { disks };

        disk_list.populate_disks_blocking();
        disk_list.sort_disks();

        disk_list
    }

    fn sort_disks(&mut self) {
        unsafe {
            self.disks
                .sortUsingSelector(sel!(localizedStandardCompare:));
        }
    }

    fn populate_disks_blocking(&mut self) {
        let Some(session) = (unsafe { DASession::new(kCFAllocatorDefault) }) else {
            return;
        };

        let disks_ptr = &self.disks as *const _ as *mut c_void;
        let _callback_guard = DiskAppearedCallbackGuard::register(&session, disks_ptr);

        let Some(run_loop) = CFRunLoop::current() else {
            return;
        };

        let Some(mode) = (unsafe { kCFRunLoopDefaultMode }) else {
            return;
        };

        unsafe { DASession::schedule_with_run_loop(&session, &run_loop, mode) };
        run_loop.stop();
        CFRunLoop::run_in_mode(unsafe { kCFRunLoopDefaultMode }, 0.05, false);
    }
}

// DRIVELIST

trait DeviceDescriptorFromDiskDescription {
    fn from_disk_description(disk_bsd_name: String, disk_description: &CFDictionary) -> Self;
}

impl DeviceDescriptorFromDiskDescription for DeviceDescriptor {
    fn from_disk_description(disk_bsd_name: String, disk_description: &CFDictionary) -> Self {
        let device_protocol =
            disk_description.get_string(unsafe { kDADiskDescriptionDeviceProtocolKey });
        let block_size =
            disk_description.get_number(unsafe { kDADiskDescriptionMediaBlockSizeKey });

        let is_internal = disk_description
            .get_number(unsafe { kDADiskDescriptionDeviceInternalKey })
            .map(|n| n.boolValue())
            .unwrap_or(false);

        let is_removable = disk_description
            .get_number(unsafe { kDADiskDescriptionMediaRemovableKey })
            .map(|n| n.boolValue())
            .unwrap_or(false);

        let is_ejectable = disk_description
            .get_number(unsafe { kDADiskDescriptionMediaEjectableKey })
            .map(|n| n.boolValue())
            .unwrap_or(false);

        let mut device = DeviceDescriptor::default();

        // Determine partition table type
        if let Some(media_content) =
            disk_description.get_string(unsafe { kDADiskDescriptionMediaContentKey })
        {
            if GUID_PARTITION_SCHEME.with(|scheme| media_content.isEqualToString(scheme)) {
                device.partition_table_type = Some("gpt".to_string());
            } else if FDISK_PARTITION_SCHEME.with(|scheme| media_content.isEqualToString(scheme)) {
                device.partition_table_type = Some("mbr".to_string());
            }
        }

        device.enumerator = "DiskArbitration".to_string();
        device.bus_type = device_protocol.as_ref().map(|s| s.to_string());
        device.bus_version = None;
        device.device = format!("/dev/{}", disk_bsd_name);
        device.device_path = disk_description
            .get_string(unsafe { kDADiskDescriptionBusPathKey })
            .map(|p| p.to_string());
        device.raw = format!("/dev/r{}", disk_bsd_name);

        device.description = disk_description
            .get_string(unsafe { kDADiskDescriptionMediaNameKey })
            .map(|desc| desc.to_string())
            .unwrap_or_default();

        device.error = None;

        // NOTE: Not sure if kDADiskDescriptionMediaBlockSizeKey returns
        // the physical or logical block size since both values are equal
        // on my machine
        //
        // The can be checked with the following command:
        //      diskutil info / | grep "Block Size"
        if let Some(bs) = block_size {
            let block_size_value = bs.unsignedIntValue();
            device.block_size = block_size_value;
            device.logical_block_size = block_size_value;
        }

        device.size = disk_description
            .get_number(unsafe { kDADiskDescriptionMediaSizeKey })
            .map(|n| n.unsignedLongValue())
            .unwrap_or(0);

        device.is_readonly = !disk_description
            .get_number(unsafe { kDADiskDescriptionMediaWritableKey })
            .map(|n| n.boolValue())
            .unwrap_or(false);

        device.is_system = is_internal && !is_removable;

        device.is_virtual = device_protocol
            .as_ref()
            .map(|p| VIRTUAL_INTERFACE.with(|vi| p.isEqualToString(vi)))
            .unwrap_or(false);

        device.is_removable = is_removable || is_ejectable;

        // Check if it's an SD card by examining the media icon
        device.is_card = disk_description
            .get_cfdict(unsafe { kDADiskDescriptionMediaIconKey })
            .and_then(|media_icon_dict| {
                IO_BUNDLE_RESOURCE_FILE.with(|key| media_icon_dict.get_cfstring(key))
            })
            .map(|icon| icon.to_string() == "SD.icns")
            .unwrap_or(false);

        // NOTE: Not convinced that these bus types should result
        // in device.is_scsi = true, it is rather "not usb or sd drive" bool
        // But the old implementation was like this so kept it this way
        device.is_scsi = device_protocol
            .as_ref()
            .map(|device| scsi_type_matches(device))
            .unwrap_or(false);

        device.is_uas = None;

        device
    }
}

pub(crate) fn drive_list() -> anyhow::Result<Vec<DeviceDescriptor>> {
    let Some(session) = (unsafe { DASession::new(kCFAllocatorDefault) }) else {
        anyhow::bail!("Failed to create DiskArbitration session");
    };

    let disk_list = DiskList::new();
    let mut device_list: Vec<DeviceDescriptor> = Vec::with_capacity(disk_list.disks.len());
    let mut device_map: HashMap<String, usize> = HashMap::with_capacity(disk_list.disks.len());

    for disk_bsd_name in &disk_list.disks {
        // Use Rust string check instead of NSPredicate regex for better performance
        if is_partition_name(&disk_bsd_name) {
            continue;
        }

        let disk_bsd_name_utf8 = disk_bsd_name.UTF8String();

        let Some(disk) = (unsafe {
            let name_ptr = NonNull::new(disk_bsd_name_utf8 as *mut c_char);
            name_ptr.and_then(|ptr| DADisk::from_bsd_name(kCFAllocatorDefault, &session, ptr))
        }) else {
            continue;
        };

        let Some(disk_description) = (unsafe { disk.description() }) else {
            continue;
        };

        let Some(disk_name_string) = disk_bsd_name_utf8.to_string() else {
            continue;
        };

        let device = DeviceDescriptor::from_disk_description(disk_name_string, &disk_description);

        // Map device path to its index in device_list for O(1) lookups later.
        let next_idx = device_list.len();
        device_map.insert(device.device.clone(), next_idx);

        device_list.push(device);
    }

    let volume_keys =
        unsafe { NSArray::from_slice(&[NSURLVolumeNameKey, NSURLVolumeLocalizedNameKey]) };

    let Some(volume_paths) = NSFileManager::defaultManager()
        .mountedVolumeURLsIncludingResourceValuesForKeys_options(
            Some(&volume_keys),
            NSVolumeEnumerationOptions(0),
        )
    else {
        return Ok(device_list);
    };

    for path in &volume_paths {
        let Some(disk) =
            (unsafe { DADisk::from_volume_path(kCFAllocatorDefault, &session, path.as_ref()) })
        else {
            continue;
        };

        let Some(partition_bsdname) = unsafe { disk.bsd_name() }.to_string() else {
            continue;
        };

        // Safely extract disk name from partition name (e.g., "disk0s1" -> "disk0")
        // Uses strip_prefix to avoid panics on unexpected BSD names
        let disk_bsdname = if let Some(rest) = partition_bsdname.strip_prefix("disk") {
            let disk_num_len = rest.find('s').unwrap_or(rest.len());
            format!("disk{}", &rest[..disk_num_len])
        } else {
            // Fallback: use the whole name if it doesn't match expected pattern
            partition_bsdname.clone()
        };

        let Some(mount_path) = path.path().and_then(|it| it.UTF8String().to_string()) else {
            continue;
        };

        let mut volume_name: Option<Retained<AnyObject>> = None;

        let Ok(_) = (unsafe {
            path.getResourceValue_forKey_error(&mut volume_name, NSURLVolumeLocalizedNameKey)
        }) else {
            continue;
        };

        let Some(name_any) = volume_name else {
            continue;
        };

        let Ok(name_str) = name_any.downcast::<NSString>() else {
            continue;
        };

        let Some(label) = name_str.UTF8String().to_string() else {
            continue;
        };

        if let Some(&idx) = device_map.get(&format!("/dev/{}", disk_bsdname)) {
            device_list[idx]
                .mountpoints
                .push(MountPoint::new(mount_path));
            device_list[idx].mountpoint_labels.push(label);
        }
    }

    Ok(device_list)
}
