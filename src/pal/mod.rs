#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

use crate::DeviceDescriptor;

#[cfg(target_os = "windows")]
pub(crate) fn drive_list() -> anyhow::Result<Vec<DeviceDescriptor>> {
    use std::{
        mem::{size_of, zeroed},
        ptr::null_mut,
    };
    use windows::*;

    use winapi::um::{
        handleapi::INVALID_HANDLE_VALUE,
        setupapi::{
            SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsA,
            DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, SP_DEVINFO_DATA,
        },
        winioctl::GUID_DEVINTERFACE_DISK,
    };

    let mut drives: Vec<DeviceDescriptor> = Vec::new();

    unsafe {
        let h_device_info = SetupDiGetClassDevsA(
            &GUID_DEVINTERFACE_DISK,
            null_mut(),
            null_mut(),
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        );

        if h_device_info != INVALID_HANDLE_VALUE {
            let mut i = 0;
            let mut device_info_data: SP_DEVINFO_DATA = zeroed();
            device_info_data.cbSize = size_of::<SP_DEVINFO_DATA>() as _;

            while SetupDiEnumDeviceInfo(h_device_info, i, &mut device_info_data) != 0 {
                let enumerator_name = get_enumerator_name(h_device_info, &mut device_info_data);
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

                get_detail_data(&mut item, h_device_info, &mut device_info_data);
                let bt = item.bus_type.clone().unwrap_or("UNKNOWN".to_string());
                item.is_system = item.is_system || is_system_device(&item);
                item.is_card = ["SDCARD", "MMC"].contains(&bt.as_str());
                item.is_uas = Some(&item.enumerator == "SCSI" && bt == "USB");
                item.is_virtual = item.is_virtual || bt == "VIRTUAL" || bt == "FILEBACKEDVIRTUAL";
                drives.push(item);
                i += 1;
            }
        }

        SetupDiDestroyDeviceInfoList(h_device_info);
    }

    Ok(drives)
}

#[cfg(target_os = "linux")]
pub(crate) fn drive_list() -> anyhow::Result<Vec<DeviceDescriptor>> {
    linux::lsblk()
}

#[cfg(target_os = "macos")]
pub(crate) fn drive_list() -> anyhow::Result<Vec<DeviceDescriptor>> {
    macos::diskutil()
}
