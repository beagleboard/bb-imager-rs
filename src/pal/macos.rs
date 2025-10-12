use core::ffi::c_void;
use std::ffi::{c_char, CStr};
use std::ptr::NonNull;

use crate::device::DeviceDescriptor;
use crate::MountPoint;
use objc2::runtime::AnyObject;
use objc2::{rc::Retained, sel};
use objc2_core_foundation::{
    kCFAllocatorDefault, kCFRunLoopDefaultMode, CFDictionary, CFRetained, CFRunLoop, CFString,
};
use objc2_disk_arbitration::{
    kDADiskDescriptionBusPathKey, kDADiskDescriptionDeviceInternalKey,
    kDADiskDescriptionDeviceProtocolKey, kDADiskDescriptionMediaBlockSizeKey,
    kDADiskDescriptionMediaContentKey, kDADiskDescriptionMediaEjectableKey,
    kDADiskDescriptionMediaIconKey, kDADiskDescriptionMediaNameKey,
    kDADiskDescriptionMediaRemovableKey, kDADiskDescriptionMediaSizeKey,
    kDADiskDescriptionMediaWritableKey, DADisk, DARegisterDiskAppearedCallback, DASession,
    DAUnregisterCallback,
};
use objc2_foundation::{
    NSArray, NSFileManager, NSMutableArray, NSNumber, NSPredicate, NSString,
    NSURLVolumeLocalizedNameKey, NSURLVolumeNameKey, NSVolumeEnumerationOptions,
};

// DISKLIST

unsafe extern "C-unwind" fn append_disk(disk: NonNull<DADisk>, context: *mut c_void) {
    if context.is_null() {
        return;
    }

    let disks = context.cast::<NSMutableArray<NSString>>();
    let bsd_name = unsafe { disk.as_ref().bsd_name() };

    if bsd_name.is_null() {
        return;
    }

    let Ok(name) = unsafe { CStr::from_ptr(bsd_name) }.to_str() else {
        return;
    };

    let string = NSString::from_str(name);

    unsafe {
        disks.as_ref().unwrap().addObject(&*string);
    }
}

struct DiskList {
    disks: Retained<NSMutableArray<NSString>>,
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
        unsafe {
            let Some(session) = DASession::new(kCFAllocatorDefault) else {
                return;
            };

            let callback =
                Some(append_disk as unsafe extern "C-unwind" fn(NonNull<DADisk>, *mut c_void));
            let array_ptr = NonNull::from(&*self.disks).as_ptr() as *mut c_void;

            DARegisterDiskAppearedCallback(&session, None, callback, array_ptr);

            let Some(run_loop) = CFRunLoop::current() else {
                return;
            };

            let Some(mode) = kCFRunLoopDefaultMode else {
                return;
            };

            DASession::schedule_with_run_loop(&session, &run_loop, mode);

            run_loop.stop();
            CFRunLoop::run_in_mode(kCFRunLoopDefaultMode, 0.05, false);

            let callback_ptr = NonNull::new_unchecked(callback.unwrap() as *mut c_void);
            DAUnregisterCallback(&session, callback_ptr, array_ptr);
            // session is released automatically when going out of scope
        }
    }
}

// DRIVELIST

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
                None
            } else {
                let ptr = NonNull::new_unchecked(value as *mut CFDictionary);
                Some(CFRetained::retain(ptr))
            }
        }
    }

    fn get_cfstring(&self, key: &CFString) -> Option<CFRetained<CFString>> {
        unsafe {
            let value = self.value(key as *const _ as *const c_void);
            if value.is_null() {
                None
            } else {
                let ptr = NonNull::new_unchecked(value as *mut CFString);
                Some(CFRetained::retain(ptr))
            }
        }
    }

    fn get_number(&self, key: &CFString) -> Option<Retained<NSNumber>> {
        unsafe {
            let value = self.value(key as *const _ as *const c_void);
            if value.is_null() {
                None
            } else {
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
                Some(Retained::retain(value as *mut NSString)?)
            }
        }
    }
}

trait DeviceDescriptorFromDiskDescription {
    fn from_disk_description(disk_bsd_name: String, disk_description: &CFDictionary) -> Self;
}

impl DeviceDescriptorFromDiskDescription for DeviceDescriptor {
    fn from_disk_description(disk_bsd_name: String, disk_description: &CFDictionary) -> Self {
        let device_protocol_key = unsafe { kDADiskDescriptionDeviceProtocolKey };
        let device_protocol = disk_description.get_string(device_protocol_key);

        let block_size_key = unsafe { kDADiskDescriptionMediaBlockSizeKey };
        let block_size = disk_description.get_number(block_size_key);

        let internal_key = unsafe { kDADiskDescriptionDeviceInternalKey };
        let is_internal = disk_description
            .get_number(internal_key)
            .map(|n| n.boolValue())
            .unwrap_or(false);

        let removable_key = unsafe { kDADiskDescriptionMediaRemovableKey };
        let is_removable = disk_description
            .get_number(removable_key)
            .map(|n| n.boolValue())
            .unwrap_or(false);

        let ejectable_key = unsafe { kDADiskDescriptionMediaEjectableKey };
        let is_ejectable = disk_description
            .get_number(ejectable_key)
            .map(|n| n.boolValue())
            .unwrap_or(false);

        let mut device = DeviceDescriptor::default();

        // Determine partition table type
        let content_key = unsafe { kDADiskDescriptionMediaContentKey };
        if let Some(media_content) = disk_description.get_string(content_key) {
            let guid_partition = NSString::from_str("GUID_partition_scheme");
            let fdisk_partition = NSString::from_str("FDisk_partition_scheme");

            if media_content.isEqualToString(&guid_partition) {
                device.partition_table_type = Some("gpt".to_string());
            } else if media_content.isEqualToString(&fdisk_partition) {
                device.partition_table_type = Some("mbr".to_string());
            }
        }

        device.enumerator = "DiskArbitration".to_string();

        device.bus_type = device_protocol.as_ref().map(|p| unsafe {
            let utf8 = p.UTF8String();
            if utf8.is_null() {
                String::new()
            } else {
                CStr::from_ptr(utf8).to_string_lossy().into_owned()
            }
        });

        device.bus_version = None;
        device.device = format!("/dev/{}", disk_bsd_name);

        let bus_path_key = unsafe { kDADiskDescriptionBusPathKey };
        device.device_path = disk_description.get_string(bus_path_key).map(|p| unsafe {
            let utf8 = p.UTF8String();
            if utf8.is_null() {
                String::new()
            } else {
                CStr::from_ptr(utf8).to_string_lossy().into_owned()
            }
        });

        device.raw = format!("/dev/r{}", disk_bsd_name);

        let name_key = unsafe { kDADiskDescriptionMediaNameKey };
        device.description = disk_description
            .get_string(name_key)
            .map(|desc| unsafe {
                let utf8 = desc.UTF8String();
                if utf8.is_null() {
                    String::new()
                } else {
                    CStr::from_ptr(utf8).to_string_lossy().into_owned()
                }
            })
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

        let size_key = unsafe { kDADiskDescriptionMediaSizeKey };
        device.size = disk_description
            .get_number(size_key)
            .map(|n| n.unsignedLongValue())
            .unwrap_or(0);

        let writable_key = unsafe { kDADiskDescriptionMediaWritableKey };
        device.is_readonly = !disk_description
            .get_number(writable_key)
            .map(|n| n.boolValue())
            .unwrap_or(false);

        device.is_system = is_internal && !is_removable;

        device.is_virtual = device_protocol
            .as_ref()
            .map(|p| {
                let virtual_interface = NSString::from_str("Virtual Interface");
                p.isEqualToString(&virtual_interface)
            })
            .unwrap_or(false);

        device.is_removable = is_removable || is_ejectable;

        // Check if it's an SD card by examining the media icon
        let icon_key = unsafe { kDADiskDescriptionMediaIconKey };
        device.is_card = disk_description
            .get_cfdict(icon_key)
            .and_then(|media_icon_dict| {
                let key = CFString::from_str("IOBundleResourceFile");
                media_icon_dict.get_cfstring(&key)
            })
            .map(|icon| icon.to_string() == "SD.icns")
            .unwrap_or(false);

        // NOTE: Not convinced that these bus types should result
        // in device.is_scsi = true, it is rather "not usb or sd drive" bool
        // But the old implementation was like this so kept it this way
        device.is_scsi = device_protocol
            .as_ref()
            .map(|p| {
                let sata = NSString::from_str("SATA");
                let scsi = NSString::from_str("SCSI");
                let ata = NSString::from_str("ATA");
                let ide = NSString::from_str("IDE");
                let pci = NSString::from_str("PCI");
                let scsi_types: Retained<NSArray<NSString>> =
                    NSArray::from_slice(&[&sata, &scsi, &ata, &ide, &pci]);
                scsi_types.containsObject(&p)
            })
            .unwrap_or(false);

        device.is_usb = device_protocol
            .as_ref()
            .map(|p| {
                let usb = NSString::from_str("USB");
                p.isEqualToString(&usb)
            })
            .unwrap_or(false);

        device.is_uas = None;

        device
    }
}

pub(crate) fn drive_list() -> anyhow::Result<Vec<DeviceDescriptor>> {
    let mut device_list: Vec<DeviceDescriptor> = Vec::new();

    let Some(session) = (unsafe { DASession::new(kCFAllocatorDefault) }) else {
        anyhow::bail!("Failed to create DiskArbitration session");
    };

    let dl = DiskList::new();

    for disk_bsd_name in &dl.disks {
        let predicate_format = NSString::from_str("SELF MATCHES %@");
        let argument = NSString::from_str(r"^disk\d+s\d+$");
        let arguments: Retained<NSArray> = NSArray::from_slice(&[&argument]);
        let partition_regex = unsafe {
            NSPredicate::predicateWithFormat_argumentArray(&predicate_format, Some(&arguments))
        };

        let is_disk_partition = unsafe { partition_regex.evaluateWithObject(Some(&disk_bsd_name)) };

        if is_disk_partition {
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

        let disk_name_string = unsafe {
            CStr::from_ptr(disk_bsd_name_utf8)
                .to_string_lossy()
                .into_owned()
        };

        let device = DeviceDescriptor::from_disk_description(disk_name_string, &disk_description);
        device_list.push(device);
        // disk and disk_description are released automatically when going out of scope
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
        let disk = match unsafe { DADisk::from_volume_path(kCFAllocatorDefault, &session, path.as_ref()) } {
            Some(d) => d,
            None => continue,
        };

        let bsdname_char = unsafe { disk.bsd_name() };
        if bsdname_char.is_null() {
            continue;
        }
        let partition_bsdname = unsafe { CStr::from_ptr(bsdname_char).to_string_lossy().into_owned() };

        let disk_len = partition_bsdname[5..]
            .find('s')
            .map(|i| i + 5)
            .unwrap_or(partition_bsdname.len());

        let disk_bsdname = partition_bsdname[..disk_len].to_string();

        let mount_path = unsafe {
            let Some(path_str) = path.path() else {
                continue;
            };
            let utf8 = path_str.UTF8String();
            CStr::from_ptr(utf8).to_string_lossy().to_string()
        };

        let mut volume_name: Option<Retained<AnyObject>> = None;

        let success = unsafe {
            path.getResourceValue_forKey_error(&mut volume_name, NSURLVolumeLocalizedNameKey)
        };
        if success.is_err() {
           continue;
        }

        let name_any = match volume_name {
            Some(n) => n,
            None => continue,
        };

        let name_str = match name_any.downcast::<NSString>() {
            Ok(s) => s,
            Err(_) => continue,
        };

        let label = unsafe {
            let utf8 = name_str.UTF8String();
            CStr::from_ptr(utf8).to_string_lossy().to_string()
        };

        if let Some(dd) = device_list.iter_mut().find(|dd| dd.device == format!("/dev/{}", disk_bsdname)) {
            dd.mountpoints.push(MountPoint::new(mount_path));
            dd.mountpoint_labels.push(label);
        }
    }

    Ok(device_list)
}
