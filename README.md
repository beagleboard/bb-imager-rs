# Readme
This is basically a Rust implementation of [Balena's drivelist](https://github.com/balena-io-modules/drivelist).
At the moment, I don't own a Mac machine, so it only supports:

 - Windows
 - Linux

[![crates.io](https://img.shields.io/crates/v/rs-drivelist?label=latest)](https://crates.io/crates/rs-drivelist) ![MSRV](https://img.shields.io/badge/rustc-1.59+-ab6000.svg) ![MIT or Apache 2.0 licensed](https://img.shields.io/crates/l/actix-web.svg)

# Preparing

Issue command at your root project directory:

    cargo add "rs-drivelist"
  Or edit your `Cargo.toml` file, and add this line:

    rs-drivelist = "0.9.0"

## Usage

This library exports one function: rs_drivelist::drive_list() which returns a `Result` of `Vec<DeviceDescriptor>`

## Windows Output

    [{
           "enumerator": "SCSI",
           "busType": "NVME",
           "busVersion": "2.0",
           "device": "\\\\.\\PhysicalDrive0",
           "devicePath": null,
           "raw": "\\\\.\\PhysicalDrive0",
           "description": "SKHynix_HFM512GDHTNI-87A0B",
           "error": null,
           "partitionTableType": "gpt",
           "size": 512110190592,
           "blockSize": 4096,
           "logicalBlockSize": 512,
           "mountpoints": [
             {
                "path": "C:\\",
                "label": null,
                "totalBytes": 136773103616,
                "availableBytes": 24087683072
             },
             {
                 "path": "D:\\",
                 "label": null,
                 "totalBytes": 218398453760,
                 "availableBytes": 35988631552
             }
          ],
          "isReadOnly": false,
          "isSystem": true,
          "isCard": false,
          "isSCSI": false,
          "isUSB": false,
          "isVirtual": false,
          "isRemovable": false,
          "isUAS": false
    }]

## Linux Output

    [{
        "enumerator": "lsblk:json",
        "busType": "NVME",
        "busVersion": null,
        "device": "/dev/nvme0n1",
        "devicePath": "/dev/disk/by-path/pci-0000:02:00.0-nvme-1",
        "raw": "/dev/nvme0n1",
        "description": " SKHynix_HFM512GDHTNI-87A0B SYSTEM_DRV, Mazter, Home, WINRE_DRV",
        "error": null,
        "partitionTableType": "gpt",
        "size": 512110190592,
        "blockSize": 512,
        "logicalBlockSize": 512,
        "mountpoints": [
          {
            "path": "/boot/efi",
            "label": "SYSTEM_DRV",
            "totalBytes": 583942144,
            "availableBytes": 541696000
          },
          {
            "path": "[SWAP]",
            "label": null,
            "totalBytes": null,
            "availableBytes": null
          },
          {
            "path": "/",
            "label": null,
            "totalBytes": 67317620736,
            "availableBytes": 47072321536
          },
          {
            "path": "/home",
            "label": "Home",
            "totalBytes": 67050090496,
            "availableBytes": 9986170880
          }
        ],
        "isReadOnly": false,
        "isSystem": true,
        "isCard": false,
        "isSCSI": false,
        "isUSB": false,
        "isVirtual": false,
        "isRemovable": false,
        "isUAS": null
    }]

Already added support for 32 bit OSes.

## Donation
My main laptop I used to start this project is broken, and I'm using a Celeron N2840 with unupgradable soldered 2GB RAM.
Your donation will be very much appreciated, as my bank account couldn't bought me a proper machine.
Visit me on my [Ko-fi account](https://ko-fi.com/ir1keren)