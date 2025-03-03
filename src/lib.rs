#[allow(non_snake_case)]
#[allow(non_snake_case)]
pub mod device;
#[cfg(target_os = "windows")]
pub(crate)mod windows;
#[cfg(target_os="linux")]
pub(crate)mod linux;
#[cfg(target_os = "macos")]
pub(crate)mod macos;

use device::DeviceDescriptor;

#[cfg(target_os = "windows")]
pub fn drive_list()->anyhow::Result<Vec<DeviceDescriptor>>
{
    use windows::*;
    use std::{ptr::null_mut, mem::{zeroed,size_of}};

    use winapi::um::{setupapi::{SetupDiGetClassDevsA, DIGCF_PRESENT, DIGCF_DEVICEINTERFACE, SP_DEVINFO_DATA, SetupDiEnumDeviceInfo, SetupDiDestroyDeviceInfoList}, winioctl::GUID_DEVINTERFACE_DISK, handleapi::INVALID_HANDLE_VALUE};

    let mut drives:Vec<DeviceDescriptor>=Vec::new();

    unsafe {
        let h_device_info=SetupDiGetClassDevsA(&GUID_DEVINTERFACE_DISK, null_mut(), null_mut(), DIGCF_PRESENT|DIGCF_DEVICEINTERFACE);

        if h_device_info!=INVALID_HANDLE_VALUE {
            let mut i=0;
            let mut device_info_data:SP_DEVINFO_DATA=zeroed();
            device_info_data.cbSize=size_of::<SP_DEVINFO_DATA>() as _;
    
            while SetupDiEnumDeviceInfo(h_device_info, i, &mut device_info_data)!=0
            {
                let enumerator_name=get_enumerator_name(h_device_info, &mut device_info_data);
                let friendly_name=get_friendly_name(h_device_info, &mut device_info_data);
    
                if friendly_name.is_empty() {
                    continue;
                }

                let mut item=DeviceDescriptor{
                    description:friendly_name.clone(),
                    enumerator:enumerator_name.clone(),
                    isUSB:is_usb_drive(&enumerator_name),
                    isRemovable:is_removable(h_device_info, &mut device_info_data),
                    ..Default::default()
                };
    
                get_detail_data(&mut item,h_device_info, &mut device_info_data);
                let bt=item.busType.clone().unwrap_or("UNKNOWN".to_string());
                item.isSystem=item.isSystem || is_system_device(&item);
                item.isCard=["SDCARD","MMC"].contains(&bt.as_str());
                item.isUAS=Some(&item.enumerator=="SCSI" && bt=="USB");
                item.isVirtual = item.isVirtual || bt == "VIRTUAL" || bt == "FILEBACKEDVIRTUAL";
                drives.push(item);
                i+=1;
            }
        }

        SetupDiDestroyDeviceInfoList(h_device_info);
    }

    Ok(drives)
}

#[cfg(target_os = "linux")]
pub fn drive_list()->anyhow::Result<Vec<DeviceDescriptor>> {
    use linux::lsblk;

    lsblk()
}

#[cfg(target_os = "macos")]
pub fn drive_list()->anyhow::Result<Vec<DeviceDescriptor>> {
    macos::diskutil()
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
