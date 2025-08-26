use std::ffi::{CString, OsStr};
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;

use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_REMOVAL_POLICY_EXPECT_ORDERLY_REMOVAL, CM_REMOVAL_POLICY_EXPECT_SURPRISE_REMOVAL,
    DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, HDEVINFO, SETUP_DI_REGISTRY_PROPERTY,
    SP_DEVICE_INTERFACE_DATA, SP_DEVICE_INTERFACE_DETAIL_DATA_W, SP_DEVINFO_DATA,
    SPDRP_ENUMERATOR_NAME, SPDRP_FRIENDLYNAME, SPDRP_REMOVAL_POLICY, SetupDiDestroyDeviceInfoList,
    SetupDiEnumDeviceInfo, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW,
    SetupDiGetDeviceInterfaceDetailW, SetupDiGetDeviceRegistryPropertyW,
};
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Storage::FileSystem::{
    BusType1394, BusTypeAta, BusTypeAtapi, BusTypeFibre, BusTypeFileBackedVirtual, BusTypeMmc,
    BusTypeNvme, BusTypeRAID, BusTypeSCM, BusTypeSas, BusTypeSata, BusTypeScsi, BusTypeSd,
    BusTypeSsa, BusTypeUfs, BusTypeUnknown, BusTypeUsb, BusTypeVirtual, BusTypeiScsi,
    FILE_SHARE_READ, GetDiskFreeSpaceW, GetDriveTypeA, GetLogicalDrives, GetVolumePathNameW,
    IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS, STORAGE_BUS_TYPE,
};
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::System::Ioctl::{
    DISK_GEOMETRY_EX, DRIVE_LAYOUT_INFORMATION_EX, GUID_DEVINTERFACE_DISK,
    IOCTL_DISK_GET_DRIVE_GEOMETRY_EX, IOCTL_DISK_GET_DRIVE_LAYOUT_EX, IOCTL_DISK_IS_WRITABLE,
    IOCTL_STORAGE_GET_DEVICE_NUMBER, IOCTL_STORAGE_QUERY_PROPERTY, PARTITION_INFORMATION_EX,
    PARTITION_STYLE_GPT, PARTITION_STYLE_MBR, PropertyStandardQuery,
    STORAGE_ACCESS_ALIGNMENT_DESCRIPTOR, STORAGE_ADAPTER_DESCRIPTOR, STORAGE_DEVICE_NUMBER,
    STORAGE_PROPERTY_QUERY, StorageAccessAlignmentProperty, StorageAdapterProperty,
    VOLUME_DISK_EXTENTS,
};
use windows::Win32::System::WindowsProgramming::{DRIVE_FIXED, DRIVE_REMOVABLE};
use windows::core::{PCSTR, PCWSTR};

use crate::{DeviceDescriptor, MountPoint};

pub(crate) fn drive_list() -> anyhow::Result<Vec<DeviceDescriptor>> {
    let mut drives: Vec<DeviceDescriptor> = Vec::new();

    let h_device_info = unsafe {
        SetupDiGetClassDevsW(
            Some(&GUID_DEVINTERFACE_DISK),
            None,
            None,
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )
    }?;

    if !h_device_info.is_invalid() {
        let mut device_info_data = SP_DEVINFO_DATA {
            cbSize: size_of::<SP_DEVINFO_DATA>() as _,
            ..Default::default()
        };

        for i in 0.. {
            if unsafe { SetupDiEnumDeviceInfo(h_device_info, i, &mut device_info_data) }.is_err() {
                break;
            }

            let enumerator_name = get_enumerator_name(h_device_info, &device_info_data);
            let friendly_name = get_friendly_name(h_device_info, &mut device_info_data);
            if friendly_name.is_empty() {
                continue;
            }

            let mut item = DeviceDescriptor {
                description: friendly_name.clone(),
                enumerator: enumerator_name.clone(),
                is_usb: is_usb_drive(&enumerator_name),
                is_removable: is_removable(h_device_info, &mut device_info_data),
                ..Default::default()
            };

            get_detail_data(&mut item, h_device_info, &mut device_info_data).unwrap();

            let bt = item.bus_type.clone().unwrap_or("UNKNOWN".to_string());
            item.is_card = ["SDCARD", "MMC"].contains(&bt.as_str());
            item.is_uas = Some(&item.enumerator == "SCSI" && bt == "USB");
            item.is_virtual = item.is_virtual || bt == "VIRTUAL" || bt == "FILEBACKEDVIRTUAL";
            item.is_system = item.is_system || is_system_device(&item);

            drives.push(item);
        }
    }

    unsafe { SetupDiDestroyDeviceInfoList(h_device_info) }?;

    Ok(drives)
}

fn setup_di_get_device_registry_property(
    h_dev_info: HDEVINFO,
    device_info_data: &SP_DEVINFO_DATA,
    property: SETUP_DI_REGISTRY_PROPERTY,
) -> String {
    let mut len = 0;
    let _ = unsafe {
        SetupDiGetDeviceRegistryPropertyW(
            h_dev_info,
            device_info_data,
            property,
            None,
            None,
            Some(&mut len),
        )
    };
    if len == 0 {
        return String::new();
    }

    let mut buf = vec![0u8; len as usize];

    unsafe {
        if SetupDiGetDeviceRegistryPropertyW(
            h_dev_info,
            device_info_data,
            property,
            None,
            Some(&mut buf),
            None,
        )
        .is_ok()
        {
            String::from_utf16_lossy(to_u16_str(&buf))
        } else {
            String::new()
        }
    }
}

fn get_enumerator_name(h_dev_info: HDEVINFO, device_info_data: &SP_DEVINFO_DATA) -> String {
    setup_di_get_device_registry_property(h_dev_info, device_info_data, SPDRP_ENUMERATOR_NAME)
}

fn get_friendly_name(h_dev_info: HDEVINFO, device_info_data: &SP_DEVINFO_DATA) -> String {
    setup_di_get_device_registry_property(h_dev_info, device_info_data, SPDRP_FRIENDLYNAME)
}

fn is_usb_drive(enumerator_name: &str) -> bool {
    [
        "USBSTOR",
        "UASPSTOR",
        "VUSBSTOR",
        "RTUSER",
        "CMIUCR",
        "EUCR",
        "ETRONSTOR",
        "ASUSSTPT",
    ]
    .contains(&enumerator_name)
}

fn is_removable(h_dev_info: HDEVINFO, device_info_data: &SP_DEVINFO_DATA) -> bool {
    let res = unsafe {
        let mut result = [0u8; 4];
        SetupDiGetDeviceRegistryPropertyW(
            h_dev_info,
            device_info_data,
            SPDRP_REMOVAL_POLICY,
            None,
            Some(&mut result),
            None,
        )
        .unwrap();

        u32::from_ne_bytes(result)
    };

    res == CM_REMOVAL_POLICY_EXPECT_SURPRISE_REMOVAL.0
        || res == CM_REMOVAL_POLICY_EXPECT_ORDERLY_REMOVAL.0
}

fn is_system_device(device: &DeviceDescriptor) -> bool {
    for key in ["windir", "ProgramFiles"] {
        let val = std::env::var(key).unwrap();

        for mp in device.mountpoints.iter() {
            if val.contains(&mp.path) {
                return true;
            }
        }
    }

    false
}

fn get_detail_data(
    device: &mut DeviceDescriptor,
    h_dev_info: HDEVINFO,
    device_info_data: &SP_DEVINFO_DATA,
) -> anyhow::Result<()> {
    for i in 0.. {
        let mut device_interface_data = SP_DEVICE_INTERFACE_DATA {
            cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as _,
            ..Default::default()
        };
        let res = unsafe {
            SetupDiEnumDeviceInterfaces(
                h_dev_info,
                Some(device_info_data),
                &GUID_DEVINTERFACE_DISK,
                i,
                &mut device_interface_data,
            )
        };
        if res.is_err() {
            break;
        }

        let p = get_device_path(h_dev_info, &device_interface_data).unwrap();

        let h_device = std::fs::OpenOptions::new()
            .access_mode(0)
            .share_mode(FILE_SHARE_READ.0)
            .open(&p)
            .unwrap();
        let Some(device_number) = get_device_number(HANDLE(h_device.as_raw_handle())) else {
            device.error = Some("Couldn't get device number".to_string());
            break;
        };

        device.raw = format!(r"\\.\PhysicalDrive{}", device_number);
        device.device = device.raw.clone();

        if let Err(err) = get_mount_points(device_number, &mut device.mountpoints) {
            device.error = Some(err.to_string());
            break;
        }

        let h_physical = std::fs::OpenOptions::new()
            .access_mode(0)
            .share_mode(FILE_SHARE_READ.0)
            .open(&device.device)
            .unwrap();
        if let Err(err) = get_device_size(device, HANDLE(h_physical.as_raw_handle())) {
            device.error = Some(format!(
                "Couldn't get device size: Error {}",
                err.to_string()
            ));
            break;
        }
        if let Err(err) = get_partition_table_type(device, HANDLE(h_physical.as_raw_handle())) {
            device.error = Some(format!(
                "Couldn't get device partition type: Error {}",
                err.to_string()
            ));
            break;
        }
        if let Err(err) = get_adapter_info(device, HANDLE(h_physical.as_raw_handle())) {
            device.error = Some(format!(
                "Couldn't get device adapter info: Error {}",
                err.to_string()
            ));
            break;
        }
        if let Err(err) = get_device_block_size(device, HANDLE(h_physical.as_raw_handle())) {
            device.error = Some(format!(
                "Couldn't get device block size: Error {}",
                err.to_string()
            ));
            break;
        }
        device.is_readonly = is_readonly(HANDLE(h_physical.as_raw_handle()));
    }

    Ok(())
}

fn is_readonly(h_physical: HANDLE) -> bool {
    unsafe {
        DeviceIoControl(
            h_physical,
            IOCTL_DISK_IS_WRITABLE,
            None,
            0,
            None,
            0,
            None,
            None,
        )
    }
    .is_err()
}

fn get_device_block_size(device: &mut DeviceDescriptor, h_physical: HANDLE) -> anyhow::Result<()> {
    let mut query = STORAGE_PROPERTY_QUERY::default();
    let mut descriptor = STORAGE_ACCESS_ALIGNMENT_DESCRIPTOR::default();

    query.QueryType = PropertyStandardQuery;
    query.PropertyId = StorageAccessAlignmentProperty;

    unsafe {
        DeviceIoControl(
            h_physical,
            IOCTL_STORAGE_QUERY_PROPERTY,
            Some(&mut query as *mut _ as *mut std::ffi::c_void),
            size_of::<STORAGE_PROPERTY_QUERY>() as u32,
            Some(&mut descriptor as *mut _ as *mut std::ffi::c_void),
            size_of::<STORAGE_ACCESS_ALIGNMENT_DESCRIPTOR>() as u32,
            None,
            None,
        )
    }?;

    device.block_size = descriptor.BytesPerPhysicalSector;
    device.logical_block_size = descriptor.BytesPerLogicalSector;

    Ok(())
}

fn get_adapter_info(device: &mut DeviceDescriptor, h_physical: HANDLE) -> anyhow::Result<()> {
    let mut query = STORAGE_PROPERTY_QUERY::default();
    let mut adapter_descriptor = STORAGE_ADAPTER_DESCRIPTOR::default();

    query.QueryType = PropertyStandardQuery;
    query.PropertyId = StorageAdapterProperty;

    unsafe {
        DeviceIoControl(
            h_physical,
            IOCTL_STORAGE_QUERY_PROPERTY,
            Some(&mut query as *mut _ as *mut std::ffi::c_void),
            size_of::<STORAGE_PROPERTY_QUERY>() as u32,
            Some(&mut adapter_descriptor as *mut _ as *mut std::ffi::c_void),
            size_of::<STORAGE_ADAPTER_DESCRIPTOR>() as u32,
            None,
            None,
        )
    }?;

    device.bus_type = Some(get_bus_type(adapter_descriptor.BusType.into()).to_string());
    device.bus_version = Some(format!(
        "{}.{}",
        adapter_descriptor.BusMajorVersion, adapter_descriptor.BusMinorVersion
    ));

    Ok(())
}

#[allow(non_upper_case_globals)]
const fn get_bus_type(bus_type: i32) -> &'static str {
    match STORAGE_BUS_TYPE(bus_type) {
        BusTypeUnknown => "UNKNOWN",
        BusTypeScsi => "SCSI",
        BusTypeAtapi => "ATAPI",
        BusTypeAta => "ATA",
        BusType1394 => "1394", // IEEE 1394
        BusTypeSsa => "SSA",
        BusTypeFibre => "FIBRE",
        BusTypeUsb => "USB",
        BusTypeRAID => "RAID",
        BusTypeiScsi => "iSCSI",
        BusTypeSas => "SAS", // Serial-Attached SCSI
        BusTypeSata => "SATA",
        BusTypeSd => "SDCARD", // Secure Digital (SD)
        BusTypeMmc => "MMC",   // Multimedia card
        BusTypeVirtual => "VIRTUAL",
        BusTypeFileBackedVirtual => "FILEBACKEDVIRTUAL",
        BusTypeNvme => "NVME",
        BusTypeUfs => "UFS",
        BusTypeSCM => "SCM",
        _ => "INVALID",
    }
}

fn get_partition_table_type(
    device: &mut DeviceDescriptor,
    h_physical: HANDLE,
) -> anyhow::Result<()> {
    const LSIZE: usize =
        size_of::<DRIVE_LAYOUT_INFORMATION_EX>() + 256 * size_of::<PARTITION_INFORMATION_EX>();

    let mut bytes = [0u8; LSIZE];
    unsafe {
        DeviceIoControl(
            h_physical,
            IOCTL_DISK_GET_DRIVE_LAYOUT_EX,
            None,
            0,
            Some(bytes.as_mut_ptr().cast()),
            LSIZE.try_into().unwrap(),
            None,
            None,
        )
    }?;

    let disk_layout = bytes.as_mut_ptr() as *mut DRIVE_LAYOUT_INFORMATION_EX;
    let partition_style = unsafe { std::ptr::addr_of_mut!((*disk_layout).PartitionStyle).read() };
    let partition_count = unsafe { std::ptr::addr_of_mut!((*disk_layout).PartitionCount).read() };

    if partition_style == PARTITION_STYLE_MBR.0 as u32 && ((partition_count % 4) == 0) {
        device.partition_table_type = Some("mbr".to_string());
    } else if partition_style == PARTITION_STYLE_GPT.0 as u32 {
        device.partition_table_type = Some("gpt".to_string());
    }

    Ok(())
}

fn get_device_size(
    device_descriptor: &mut DeviceDescriptor,
    h_physical: HANDLE,
) -> anyhow::Result<()> {
    let mut disk_geometry = DISK_GEOMETRY_EX::default();

    unsafe {
        DeviceIoControl(
            h_physical,
            IOCTL_DISK_GET_DRIVE_GEOMETRY_EX,
            None,
            0,
            Some(&mut disk_geometry as *mut _ as *mut std::ffi::c_void),
            size_of::<DISK_GEOMETRY_EX>() as _,
            None,
            None,
        )
    }?;

    device_descriptor.size = disk_geometry.DiskSize as u64;
    device_descriptor.block_size = disk_geometry.Geometry.BytesPerSector;

    Ok(())
}

fn get_available_volumes() -> Vec<char> {
    unsafe {
        let mut logical_drive_mask = GetLogicalDrives();
        let mut current_drive_letter = b'A';
        let mut vec_char: Vec<char> = Vec::new();

        while logical_drive_mask != 0 {
            if (logical_drive_mask & 1) != 0 {
                vec_char.push(current_drive_letter as _);
            }

            current_drive_letter += 1;
            logical_drive_mask >>= 1;
        }

        vec_char
    }
}

fn get_mount_points(device_number: u32, mount_points: &mut Vec<MountPoint>) -> anyhow::Result<()> {
    for volume_name in get_available_volumes() {
        let mut drive = MountPoint::new(format!(r"{}:\", volume_name));
        let drive_type = unsafe {
            GetDriveTypeA(PCSTR::from_raw(
                CString::new(drive.path.clone()).unwrap().as_ptr().cast(),
            ))
        };

        if drive_type != DRIVE_FIXED && drive_type != DRIVE_REMOVABLE {
            continue;
        }

        let Ok(h_logical) = std::fs::OpenOptions::new()
            .access_mode(0)
            .share_mode(FILE_SHARE_READ.0)
            .open(format!(r"\\.\{}:", volume_name))
        else {
            continue;
        };
        let Some(logical_volume_device_number) =
            get_device_number(HANDLE(h_logical.as_raw_handle()))
        else {
            continue;
        };

        if logical_volume_device_number == device_number {
            let root_path = &mut [0_u16; 261];
            let path_os: Vec<u16> = OsStr::new(&drive.path)
                .encode_wide()
                .chain(Some(0))
                .collect();

            unsafe { GetVolumePathNameW(PCWSTR::from_raw(path_os.as_ptr()), root_path)? };

            let mut sectors_per_cluster = 0;
            let mut bytes_per_sector = 0;
            let mut number_of_free_clusters = 0;
            let mut total_number_of_clusters = 0;
            unsafe {
                GetDiskFreeSpaceW(
                    PCWSTR::from_raw(root_path.as_ptr()),
                    Some(&mut sectors_per_cluster),
                    Some(&mut bytes_per_sector),
                    Some(&mut number_of_free_clusters),
                    Some(&mut total_number_of_clusters),
                )
            }?;

            let bytes_per_cluster = sectors_per_cluster as u64 * bytes_per_sector as u64;
            drive.total_bytes = Some(bytes_per_cluster * total_number_of_clusters as u64);
            drive.available_bytes = Some(bytes_per_cluster * number_of_free_clusters as u64);
            mount_points.push(drive);
        }
    }

    Ok(())
}

fn get_device_number(h_device: HANDLE) -> Option<u32> {
    let mut size = 0u32;
    let mut disk_extents = VOLUME_DISK_EXTENTS::default();

    let res = unsafe {
        DeviceIoControl(
            h_device,
            IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS,
            None,
            0,
            Some(&mut disk_extents as *mut _ as *mut std::ffi::c_void),
            size_of::<VOLUME_DISK_EXTENTS>() as _,
            Some(&mut size),
            None,
        )
    };

    if res.is_ok() {
        if disk_extents.NumberOfDiskExtents >= 2 {
            return None;
        }
    }

    let mut device_number = STORAGE_DEVICE_NUMBER::default();

    let res = unsafe {
        DeviceIoControl(
            h_device,
            IOCTL_STORAGE_GET_DEVICE_NUMBER,
            None,
            0,
            Some(&mut device_number as *mut _ as *mut std::ffi::c_void),
            size_of::<STORAGE_DEVICE_NUMBER>() as _,
            Some(&mut size),
            None,
        )
    };

    if res.is_ok() {
        Some(device_number.DeviceNumber)
    } else {
        Some(disk_extents.Extents[0].DiskNumber)
    }
}

fn get_device_path(
    h_dev_info: HDEVINFO,
    device_interface_data: &SP_DEVICE_INTERFACE_DATA,
) -> anyhow::Result<String> {
    let mut len = 0;

    let _ = unsafe {
        SetupDiGetDeviceInterfaceDetailW(
            h_dev_info,
            device_interface_data,
            None,
            0,
            Some(&mut len),
            None,
        )
    };
    assert!(len != 0);

    let mut buf: Vec<u8> = vec![0u8; len as usize];
    let device_iface_detail: *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W = buf.as_mut_ptr().cast();
    unsafe {
        std::ptr::addr_of_mut!((*device_iface_detail).cbSize)
            .write(std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32);
    }

    unsafe {
        SetupDiGetDeviceInterfaceDetailW(
            h_dev_info,
            device_interface_data,
            Some(device_iface_detail),
            len,
            None,
            None,
        )
    }?;

    let str_len = (len as usize - std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>()) / 2 + 1;
    let str =
        unsafe { std::slice::from_raw_parts((*device_iface_detail).DevicePath.as_ptr(), str_len) };

    Ok(String::from_utf16_lossy(str))
}

fn to_u16_str(arr: &[u8]) -> &[u16] {
    assert_eq!(arr.len() % 2, 0);
    let len = arr.len() / std::mem::size_of::<u16>();
    // Strip NULL
    let len = if arr[(arr.len() - 2)..] == [0, 0] {
        len - 1
    } else {
        len
    };
    unsafe { std::slice::from_raw_parts(arr.as_ptr().cast(), len) }
}
