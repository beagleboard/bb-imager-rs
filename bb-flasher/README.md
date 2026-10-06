# Introduction

This crate provides common abstractions over the different flashers to be used by applications
such as BeagleBoard Imaging Utility. It also provides traits to add more flashers which behave
similar to the pre-defined ones.

# Usage

Flash an OS image to an SD Card (requires the `sd` feature):

```toml
[dependencies]
bb-flasher = { version = "0.1", features = ["sd"] }
```

```rust,no_run
use std::path::PathBuf;
use bb_flasher::{LocalImage, sd};

let img = LocalImage::new(PathBuf::from("/tmp/abc.img.xz").into_boxed_path());
let target: sd::Target = PathBuf::from("/dev/sdb").try_into().unwrap();
let customization =
    sd::FlashingSdLinuxConfig::sysconfig(Some("beaglebone"), None, None, None, None, None, None);

sd::Flasher::new(
    img.into_image_fn(),
    sd::NONE_BOOTFS,
    None::<fn() -> std::io::Result<Box<str>>>,
    target,
    customization,
    true,
)
.flash(None, None)
.unwrap();
```

# Features

No features are enabled by default.

- `sd`: Flash Linux images to SD Cards.
- `sd_linux_udev`: Uses udev to provide GUI prompt to open SD Cards in Linux. Useful for GUI
  applications.
- `sd_macos_authopen`: Uses authopen to provide GUI prompt to open SD Cards in MacOS. Useful for
  GUI applications.
- `bcf`: Flash the main processor (CC1352P7) in BeagleConnect Freedom.
- `bcf_msp430`: Flash MSP430 in BeagleConnect Freedom, which acts as the USB to UART bridge.
- `pb2_mspm0`: Flash PocketBeagle 2 MSPM0. Needs root permissions.
- `mspm0_uart`: Flash MSPM0 over UART.
- `mspm0_i2c`: Flash MSPM0 over I2C. Linux only.
- `dfu`: Flash devices over USB DFU.
- `piped_image`: Construct images from streams (e.g. a download in progress).
- `serde`: Implement `serde::Serialize` for some types. Enabled by `sd`.
- `static`: Statically link liblzma and hidraw.
- `static_hidraw` / `shared_hidraw`: Statically or dynamically link hidraw for BeagleConnect
  Freedom.
