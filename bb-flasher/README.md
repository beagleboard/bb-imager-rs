# Introduction

This crate provides common abstractions over the different flashers to be used by applications
such as BeagleBoard Imaging Utility. It also provides traits to add more flashers which behave
similiar to the pre-defined ones

# Usage

```rust
use std::path::PathBuf;
use bb_flasher::BBFlasher;

#[tokio::main]
async fn main() {
    let img = bb_flasher::LocalImage::new("/tmp/abc.img.xz".into());
    let target = PathBuf::from("/tmp/target").try_into().unwrap();
    let customization = 
        bb_flasher::sd::FlashingSdLinuxConfig::new(true, None, None, None, None, None);

    let flasher = bb_flasher::sd::Flasher::new(img, target, customization)
        .flash(None)
        .await
        .unwrap();
}
```

# Features

- `sd`: Provide flashing Linux images to SD Cards. Enabled by **default**.
- `sd_linux_udev`: Uses udev to provide GUI prompt to open SD Cards in Linux. Useful for GUI
applications.
- `sd_macos_authopen`: Uses authopen to provide GUI prompt to open SD Cards in MacOS. Useful
for GUI applications.
- `bcf`: Provde support for flashing the main processor (CC1352P7) in BeagleConnect Freedom.
- `bcf_msp430`: Provide support for flashing MSP430 in BeagleConnect Freedom, which acts as the
USB to UART bridge.
- `pb2_mspm0`: Provides support to flash PocketBeagle 2 MSPM0. Needs root permissions.
- `pb2_mspm0_dbus`: Use bb-imager-serivce to flash PocketBeagle 2 as a normal user.
