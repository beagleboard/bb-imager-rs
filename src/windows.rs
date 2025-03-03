use std::{ptr::{null_mut}, mem::{zeroed, size_of, MaybeUninit, align_of, transmute_copy}, str::from_utf8, ffi::{CString, OsStr}, os::windows::prelude::OsStrExt};
use winapi::{um::{setupapi::{HDEVINFO, PSP_DEVINFO_DATA, SetupDiGetDeviceRegistryPropertyW, SPDRP_FRIENDLYNAME, SPDRP_REMOVAL_POLICY, SPDRP_ENUMERATOR_NAME, SP_DEVICE_INTERFACE_DATA, SetupDiEnumDeviceInterfaces, SetupDiGetDeviceInterfaceDetailW, SP_DEVICE_INTERFACE_DETAIL_DATA_W}, winioctl::{GUID_DEVINTERFACE_DISK, VOLUME_DISK_EXTENTS, IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS, IOCTL_STORAGE_GET_DEVICE_NUMBER, STORAGE_DEVICE_NUMBER, DISK_GEOMETRY_EX, IOCTL_DISK_GET_DRIVE_GEOMETRY_EX, DRIVE_LAYOUT_INFORMATION_EX, PARTITION_INFORMATION_EX, IOCTL_DISK_GET_DRIVE_LAYOUT_EX, PARTITION_STYLE_MBR, PARTITION_STYLE_GPT, STORAGE_PROPERTY_QUERY, PropertyStandardQuery, StorageAdapterProperty, IOCTL_STORAGE_QUERY_PROPERTY, StorageAccessAlignmentProperty, IOCTL_DISK_IS_WRITABLE}, handleapi::{INVALID_HANDLE_VALUE, CloseHandle}, cfgmgr32::{CM_REMOVAL_POLICY_EXPECT_SURPRISE_REMOVAL, CM_REMOVAL_POLICY_EXPECT_ORDERLY_REMOVAL}, errhandlingapi::GetLastError, fileapi::{CreateFileW, OPEN_EXISTING, GetLogicalDrives, GetDriveTypeA, CreateFileA, GetVolumePathNameW, GetDiskFreeSpaceW}, winnt::{FILE_SHARE_READ, FILE_ATTRIBUTE_NORMAL, BOOLEAN}, ioapiset::DeviceIoControl, winbase::{DRIVE_FIXED, DRIVE_REMOVABLE}, processenv::ExpandEnvironmentStringsA}, shared::{minwindef::{MAX_PATH, DWORD, WORD, BYTE}, winerror::{ERROR_NO_MORE_ITEMS, ERROR_INSUFFICIENT_BUFFER}}, ctypes::c_void};
use crate::device::*;

pub(crate)fn ansi_to_string(unsafe_utf8:&[u8])->String
{
    match from_utf8(&unsafe_utf8.iter().filter(|c| **c != 0).map(|c| *c).collect::<Vec<u8>>() as _)
    {
        Err(err)=>{
            println!("Error {}",err);
            "".to_string()
        },
        Ok(res)=>res.trim().to_string()
    }
}

#[repr(C)]
#[derive(Copy)]
#[allow(non_snake_case)]
struct STORAGE_ADAPTER_DESCRIPTOR
{
    Version:DWORD,
    Size:DWORD,
    MaximumTransferLength:DWORD,
    MaximumPhysicalPages:DWORD,
    AlignmentMask:DWORD,
    AdapterUsesPio:BOOLEAN,
    AdapterScansDown:BOOLEAN,
    CommandQueueing:BOOLEAN,
    AcceleratedTransfer:BOOLEAN,
    BusType:BOOLEAN,
    BusMajorVersion:WORD,
    BusMinorVersion:WORD,
    SrbType:BYTE,
    AddressType:BYTE
}

impl Clone for STORAGE_ADAPTER_DESCRIPTOR
{
    fn clone(&self) -> Self {
        *self
    }
}

impl Default for STORAGE_ADAPTER_DESCRIPTOR
{
    fn default() -> Self {
        unsafe {
            zeroed()
        }
    }
}

type StorageBusType=u32;
const BUS_TYPE_UNKNOWN:StorageBusType=0;
const BUS_TYPE_SCSI:StorageBusType=1;
const BUS_TYPE_ATAPI:StorageBusType=2;
const BUS_TYPE_ATA:StorageBusType=3;
const BUS_TYPE1394:StorageBusType=4;
const BUS_TYPE_SSA:StorageBusType=5;
const BUS_TYPE_FIBRE:StorageBusType=6;
const BUS_TYPE_USB:StorageBusType=7;
const BUS_TYPE_RAID:StorageBusType=8;
const BUS_TYPEI_SCSI:StorageBusType=9;
const BUS_TYPE_SAS:StorageBusType=10;
const BUS_TYPE_SATA:StorageBusType=11;
const BUS_TYPE_SD:StorageBusType=12;
const BUS_TYPE_MMC:StorageBusType=13;
const BUS_TYPE_VIRTUAL:StorageBusType=14;
const BUS_TYPE_FILE_BACKED_VIRTUAL:StorageBusType=15;
//const BusTypeSpaces:STORAGE_BUS_TYPE=16;
const BUS_TYPE_NVME:StorageBusType=17;
const BUS_TYPE_SCM:StorageBusType=18;
const BUS_TYPE_UFS:StorageBusType=19;
//const BusTypeMax:STORAGE_BUS_TYPE=20;
//const BusTypeMaxReserved:STORAGE_BUS_TYPE=0x7F;

fn get_adapter_info(device:&mut DeviceDescriptor,h_physical:*mut c_void)->bool
{
    unsafe{
        let mut query=MaybeUninit::<STORAGE_PROPERTY_QUERY>::zeroed();
        let mut adapter_descriptor=MaybeUninit::<STORAGE_ADAPTER_DESCRIPTOR>::zeroed();
        let mut size=0_u32;

        query.assume_init_mut().QueryType=PropertyStandardQuery;
        query.assume_init_mut().PropertyId=StorageAdapterProperty;

        let has_adapter_info=DeviceIoControl(h_physical,IOCTL_STORAGE_QUERY_PROPERTY,query.as_mut_ptr() as _,size_of::<STORAGE_PROPERTY_QUERY>() as u32,adapter_descriptor.as_mut_ptr() as _,size_of::<STORAGE_ADAPTER_DESCRIPTOR>() as u32,&mut size,null_mut());

        if has_adapter_info !=0 {
            let val=adapter_descriptor.assume_init_ref();
            device.busType=Some(get_bus_type(val));
            device.busVersion=Some(format!("{}.{}",val.BusMajorVersion,val.BusMinorVersion));
            return true;
        }
    }

    false
}

fn get_available_volumes() -> Vec<char>
{
    unsafe{
        let mut logical_drive_mask=GetLogicalDrives();
        let mut current_drive_letter=b'A';
        let mut vec_char:Vec<char>=Vec::new();

        while logical_drive_mask!=0
        {
            if (logical_drive_mask & 1)!=0 {
                vec_char.push(current_drive_letter as _);
            }

            current_drive_letter+=1;
            logical_drive_mask=logical_drive_mask >>1;
        }

        vec_char
    }
}

fn get_bus_type(adapter:&STORAGE_ADAPTER_DESCRIPTOR)->String
{
    match adapter.BusType as u32 {
        BUS_TYPE_UNKNOWN => "UNKNOWN",
        BUS_TYPE_SCSI=>"SCSI",
        BUS_TYPE_ATAPI=>"ATAPI",
        BUS_TYPE_ATA=>"ATA",
        BUS_TYPE1394=>"1394",  // IEEE 1394
        BUS_TYPE_SSA=>"SSA",
        BUS_TYPE_FIBRE=>"FIBRE",
        BUS_TYPE_USB=>"USB",
        BUS_TYPE_RAID=>"RAID",
        BUS_TYPEI_SCSI=>"iSCSI",
        BUS_TYPE_SAS=>"SAS",  // Serial-Attached SCSI
        BUS_TYPE_SATA=>"SATA",
        BUS_TYPE_SD=>"SDCARD",  // Secure Digital (SD)
        BUS_TYPE_MMC=>"MMC",  // Multimedia card
        BUS_TYPE_VIRTUAL=>"VIRTUAL",
        BUS_TYPE_FILE_BACKED_VIRTUAL=>"FILEBACKEDVIRTUAL",
        BUS_TYPE_NVME=>"NVME",
        BUS_TYPE_UFS=>"UFS",
        BUS_TYPE_SCM=>"SCM",
        _=>"INVALID"
    }.to_string()
}

pub(crate)fn is_system_device(device:&DeviceDescriptor)->bool
{
    unsafe {
        for sys_var in ["%windir%\0","%ProgramFiles%\0"] {
            let mut buffer:[i8;MAX_PATH]=zeroed();
            let res=ExpandEnvironmentStringsA(sys_var.as_ptr() as _, &mut buffer as _, (size_of::<u8>() * MAX_PATH) as u32);

            if res > 0 {
                let mut tmp_buffer=vec![0_u8;res as usize];
                
                for i in buffer {
                    tmp_buffer.push(i as u8);
                }

                let val=ansi_to_string(&tmp_buffer);

                for mp in device.mountpoints.iter() {
                    if val.contains(&mp.path) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
struct STORAGE_ACCESS_ALIGNMENT_DESCRIPTOR {
    Version:DWORD,
    Size:DWORD,
    BytesPerCacheLine:DWORD,
    BytesOffsetForCacheAlignment:DWORD,
    BytesPerLogicalSector:DWORD,
    BytesPerPhysicalSector:DWORD,
    BytesOffsetForSectorAlignment:DWORD
}

fn get_device_block_size(device:&mut DeviceDescriptor,h_physical:*mut c_void) -> bool
{
    unsafe {
        let mut query=MaybeUninit::<STORAGE_PROPERTY_QUERY>::zeroed();
        let mut descriptor=MaybeUninit::<STORAGE_ACCESS_ALIGNMENT_DESCRIPTOR>::zeroed();
        let mut size=0_u32;

        query.assume_init_mut().QueryType=PropertyStandardQuery;
        query.assume_init_mut().PropertyId=StorageAccessAlignmentProperty;

        let has_adapter_info=DeviceIoControl(h_physical,IOCTL_STORAGE_QUERY_PROPERTY,query.as_mut_ptr() as _,size_of::<STORAGE_PROPERTY_QUERY>() as u32,descriptor.as_mut_ptr() as _,size_of::<STORAGE_ACCESS_ALIGNMENT_DESCRIPTOR>() as u32,&mut size,null_mut());

        if has_adapter_info !=0 {
            let val=descriptor.assume_init_ref();
            device.blockSize=val.BytesPerPhysicalSector;
            device.logicalBlockSize=val.BytesPerLogicalSector;
            return true;
        }
    }

    false
}

fn get_device_number(h_device:*mut c_void) -> i32
{
    unsafe {
        let mut size=0_u32;
        let mut disk_number=-1;
        //let mut void_buffer=null_mut();
        //let mut disk_extents=MaybeUninit::<VOLUME_DISK_EXTENTS>::uninit();

        let mut disk_extents=MaybeUninit::<VOLUME_DISK_EXTENTS>::uninit();
        disk_extents.write(zeroed());
        let mut result=DeviceIoControl(h_device, IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS, null_mut(), 0, disk_extents.as_mut_ptr() as _, size_of::<VOLUME_DISK_EXTENTS>() as _, &mut size, null_mut());

        if result!=0 {
            let de=disk_extents.assume_init_ref();

            if de.NumberOfDiskExtents>=2 {
                return -1;
            }

            disk_number=de.Extents[0].DiskNumber as _;
        }

        let mut device_number=MaybeUninit::<STORAGE_DEVICE_NUMBER>::uninit();
        device_number.write(zeroed());
        
        result=DeviceIoControl(h_device, IOCTL_STORAGE_GET_DEVICE_NUMBER, null_mut(), 0, device_number.as_mut_ptr() as _, size_of::<STORAGE_DEVICE_NUMBER>() as _, &mut size, null_mut());

        if result!=0 {
            disk_number=device_number.assume_init_ref().DeviceNumber as _;
        }

        disk_number
    }
}

pub(crate)fn get_detail_data(device:&mut DeviceDescriptor,h_dev_info:HDEVINFO,device_info_data:PSP_DEVINFO_DATA)
{
    let mut h_device=INVALID_HANDLE_VALUE;
    let mut index=0_u32;

    unsafe{
        loop {
            if h_device!= INVALID_HANDLE_VALUE {
                CloseHandle(h_device);
                h_device=INVALID_HANDLE_VALUE;
            }

            let mut device_interface_data:SP_DEVICE_INTERFACE_DATA=zeroed();
            device_interface_data.cbSize=size_of::<SP_DEVICE_INTERFACE_DATA>() as _;
            
            if SetupDiEnumDeviceInterfaces(h_dev_info, device_info_data, &GUID_DEVINTERFACE_DISK, index, &mut device_interface_data) == 0 {
                let error_code=GetLastError();

                if error_code!=ERROR_NO_MORE_ITEMS {
                    panic!("SetupDiEnumDeviceInterfaces: Error {}",error_code);
                }

                break;
            } else {
                let mut size={
                    let mut required_size=MaybeUninit::<u32>::uninit();

                    if SetupDiGetDeviceInterfaceDetailW(h_dev_info, &mut device_interface_data, null_mut(), 0, required_size.as_mut_ptr(), null_mut())==0 {
                        if GetLastError()==ERROR_INSUFFICIENT_BUFFER {
                            required_size.assume_init()
                        } else {
                            panic!("Error SetupDiGetDeviceInterfaceDetailW");
                        }
                    } else {
                        0
                    }
                };
                let mut buf:Vec<u8>=Vec::with_capacity(TryInto::<usize>::try_into(size).unwrap() + align_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>()-1);
                let align_offset=buf.as_mut_ptr().align_offset(align_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>());
                let device_iface_detail =&mut *(buf.as_mut_ptr().offset(align_offset.try_into().unwrap()) as *mut MaybeUninit<SP_DEVICE_INTERFACE_DETAIL_DATA_W>);
                device_iface_detail.write(SP_DEVICE_INTERFACE_DETAIL_DATA_W {
                    cbSize: size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>().try_into().unwrap(),
                    DevicePath: [0],
                });

                if SetupDiGetDeviceInterfaceDetailW(h_dev_info, &mut device_interface_data, device_iface_detail.as_mut_ptr(), size, &mut size, null_mut())==0 {
                    println!("Error {}, Couldn't SetupDiGetDeviceInterfaceDetailW",GetLastError());
                    break;
                }

                let device_detail_data=device_iface_detail.assume_init_ref();
                
                h_device=CreateFileW(device_detail_data.DevicePath.as_ptr(), 0, FILE_SHARE_READ,  null_mut(), OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, null_mut());
                
                if h_device==INVALID_HANDLE_VALUE {
                    println!("Couldn't open handle to device: Error {}",GetLastError());
                    break;
                }

                let device_number=get_device_number(h_device);

                if device_number <0 {
                    device.error=Some("Couldn't get device number".to_string());
                    break;
                }
                
                device.raw=format!(r"\\.\PhysicalDrive{}",device_number);
                device.device=device.raw.clone();
                
                if let Err(err)=get_mount_points(device_number, &mut device.mountpoints) {
                    device.error=Some(err.to_string());
                    break;
                }

                let h_physical=CreateFileA(CString::new(device.device.clone()).unwrap().as_ptr(), 0, FILE_SHARE_READ, null_mut(), OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, null_mut());

                if h_physical==INVALID_HANDLE_VALUE {
                    device.error=Some(format!("Cannot open: {}",device.devicePath.as_ref().unwrap()));
                    break;
                }

                if !get_device_size(device,h_physical) {
                    let error_code = GetLastError();
                    device.error=Some(format!("Couldn't get disk geometry: Error {}",error_code));
                    break;
                }

                if !get_partition_table_type(device, h_physical) {
                    device.error=Some(format!("Couldn't get partition type: Error {}",GetLastError()));
                    break;
                }

                if !get_adapter_info(device, h_physical) {
                    device.error=Some(format!("Couldn't get adapter info: Error {}",GetLastError()));
                    break;
                }

                if !get_device_block_size(device, h_physical) {
                    device.error=Some(format!("Couldn't get device block size: Error {}",GetLastError()));
                    break;
                }

                device.isReadOnly=DeviceIoControl(h_physical,IOCTL_DISK_IS_WRITABLE,null_mut(),0,null_mut(),0,&mut size,null_mut())==0;
                CloseHandle(h_physical);
            }

            index+=1;
        }

        if h_device!= INVALID_HANDLE_VALUE {
            CloseHandle(h_device);
        }
    }
}

fn get_device_size(device_descriptor:&mut DeviceDescriptor,h_physical:*mut c_void)->bool
{
    unsafe{
        let mut disk_geometry=MaybeUninit::<DISK_GEOMETRY_EX>::uninit();
        disk_geometry.write(zeroed());
        let mut size=0;
        let has_disk_geometry=DeviceIoControl(h_physical, IOCTL_DISK_GET_DRIVE_GEOMETRY_EX, null_mut(), 0, disk_geometry.as_mut_ptr() as _, size_of::<DISK_GEOMETRY_EX>() as _, &mut size, null_mut());

        if has_disk_geometry!=0 {
            let dm=disk_geometry.assume_init_ref();
            device_descriptor.size=(*dm.DiskSize.QuadPart()) as u64;
            device_descriptor.blockSize=dm.Geometry.BytesPerSector;
        }

        has_disk_geometry!=0
    }
}

pub(crate)fn get_enumerator_name(h_dev_info:HDEVINFO,device_info_data:PSP_DEVINFO_DATA) -> String
{
    unsafe {
        let mut buffer:[u8;MAX_PATH]=zeroed();
        
        if SetupDiGetDeviceRegistryPropertyW(h_dev_info, device_info_data, SPDRP_ENUMERATOR_NAME, null_mut(), &mut buffer as _, (size_of::<u8>() * MAX_PATH) as _, null_mut()) != 0 {
            ansi_to_string(&buffer)
        } else {
            "".to_string()
        }
    }
}

pub(crate)fn get_friendly_name(h_dev_info:HDEVINFO,device_info_data:PSP_DEVINFO_DATA) -> String
{
    unsafe {
        let mut buffer:[u8;MAX_PATH]=zeroed();
        
        if SetupDiGetDeviceRegistryPropertyW(h_dev_info, device_info_data, SPDRP_FRIENDLYNAME, null_mut(), &mut buffer as _, (size_of::<u8>() * MAX_PATH) as _, null_mut()) != 0 {
            ansi_to_string(&buffer)
        } else {
            "".to_string()
        }
    }
}

fn get_mount_points(device_number:i32,mount_points:&mut Vec<MountPoint>)->anyhow::Result<()>
{
    unsafe {
        let mut h_logical=INVALID_HANDLE_VALUE;

        for volume_name in get_available_volumes()
        {
            if h_logical!=INVALID_HANDLE_VALUE {
                CloseHandle(h_logical);
                h_logical=INVALID_HANDLE_VALUE;
            }

            let mut drive=MountPoint::new(format!(r"{}:\",volume_name));
            let drive_type=GetDriveTypeA(CString::new(drive.path.clone()).unwrap().as_ptr());

            if drive_type!=DRIVE_FIXED && drive_type!=DRIVE_REMOVABLE {
                continue;
            }

            
            let h_logical=CreateFileA(CString::new(format!(r"\\.\{}:",volume_name)).unwrap().as_ptr(), 0, FILE_SHARE_READ, null_mut(), OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, null_mut());

            if h_logical==INVALID_HANDLE_VALUE {
                continue;
            }

            let logical_volume_device_number=get_device_number(h_logical);

            if logical_volume_device_number<0 {
                continue;
            }

            if logical_volume_device_number==device_number {
                let root_path = &mut [0_u16; 261];
                let path_os:Vec<u16>=OsStr::new(&drive.path).encode_wide().chain(Some(0)).collect();

                let mut ret=GetVolumePathNameW(path_os.as_ptr(), root_path.as_mut_ptr(), root_path.len() as _);

                if ret==0 {
                    return Err(anyhow::Error::new(std::io::Error::last_os_error()));
                }

                let mut sectors_per_cluster = 0;
                let mut bytes_per_sector = 0;
                let mut number_of_free_clusters = 0;
                let mut total_number_of_clusters = 0;
                ret = GetDiskFreeSpaceW(root_path.as_ptr(),&mut sectors_per_cluster,&mut bytes_per_sector,&mut number_of_free_clusters,&mut total_number_of_clusters);

                if ret==0 {
                    return Err(anyhow::Error::new(std::io::Error::last_os_error()));
                }

                let bytes_per_cluster = sectors_per_cluster as u64 * bytes_per_sector as u64;
                drive.totalBytes=Some(bytes_per_cluster * total_number_of_clusters as u64);
                drive.availableBytes=Some(bytes_per_cluster * number_of_free_clusters as u64);
                mount_points.push(drive);
            }
        }

        if h_logical!=INVALID_HANDLE_VALUE {
            CloseHandle(h_logical);
        }
    }

    Ok(())
}

fn get_partition_table_type(device:&mut DeviceDescriptor,h_physical:*mut c_void)->bool
{
    unsafe{
        const LSIZE: usize=size_of::<DRIVE_LAYOUT_INFORMATION_EX>() + 256 * size_of::<PARTITION_INFORMATION_EX>();
        let mut bytes:[u8;LSIZE]=zeroed();
        let mut disk_layout_size=0_u32;
        let has_disk_layout=DeviceIoControl(h_physical, IOCTL_DISK_GET_DRIVE_LAYOUT_EX, null_mut(), 0,bytes.as_mut_ptr() as _, LSIZE.try_into().unwrap(),&mut disk_layout_size, null_mut());

        if has_disk_layout==0 {
            device.error=Some(format!("NOT has_disk_layout. Error {}",GetLastError()));
            return false;
        }

        let disk_layout:DRIVE_LAYOUT_INFORMATION_EX=transmute_copy(&bytes);

        if disk_layout.PartitionStyle==PARTITION_STYLE_MBR && ((disk_layout.PartitionCount % 4)==0) {
            device.partitionTableType=Some("mbr".to_string());
        } else if disk_layout.PartitionStyle==PARTITION_STYLE_GPT {
            device.partitionTableType=Some("gpt".to_string());
        }
    }

    true
}

pub(crate)fn is_usb_drive(enumerator_name:&str) -> bool
{
    ["USBSTOR", "UASPSTOR", "VUSBSTOR","RTUSER", "CMIUCR", "EUCR","ETRONSTOR", "ASUSSTPT"].contains(&enumerator_name)
}

pub(crate)fn is_removable(h_dev_info:HDEVINFO,device_info_data:PSP_DEVINFO_DATA)->bool
{
    unsafe {
        let mut result=0_u8;
        SetupDiGetDeviceRegistryPropertyW(h_dev_info, device_info_data, SPDRP_REMOVAL_POLICY, null_mut(), &mut result as _, size_of::<u32>() as _, null_mut());

        match result as u32
        {
            CM_REMOVAL_POLICY_EXPECT_SURPRISE_REMOVAL|CM_REMOVAL_POLICY_EXPECT_ORDERLY_REMOVAL =>true,
            _=>false
        }
    }
}