# bb-imager-cli

A streamlined tool for creating, flashing, and managing OS images for [BeagleBoard.org](https://www.beagleboard.org/) devices. This is the command line version of the [BeagleBoard Imaging Utility](https://github.com/beagleboard/bb-imager-rs).

# Installation

```shell
cargo install bb-imager-cli
```

On Linux, building requires the udev, liblzma and hidapi development packages, e.g. `libudev-dev liblzma-dev libhidapi-dev` on Debian/Ubuntu or `systemd-devel xz-devel hidapi-devel` on Fedora.

Prebuilt packages are available from the [releases page](https://github.com/beagleboard/bb-imager-rs/releases).

On Linux (x86_64, aarch64 and armv7), [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) can install the prebuilt binary instead of building from source:

```shell
cargo binstall bb-imager-cli
```

# Usage

List SD Cards:

```shell
bb-imager-cli list-destinations sd
```

Flash an image to an SD Card, with optional customization:

```shell
bb-imager-cli flash sd image.img.xz /dev/sdX --hostname beaglebone --user-name debian --user-password temppwd
```

Format an SD Card:

```shell
bb-imager-cli format /dev/sdX
```

Generate shell completions:

```shell
bb-imager-cli generate-completion bash > bb-imager-cli.bash
```

Run `bb-imager-cli --help` or `bb-imager-cli <command> --help` for all commands and options.

# Features

Enabled by default:

- `bcf_cc1352p7`: Flash the main processor (CC1352P7) in BeagleConnect Freedom.
- `zepto_uart`: Flash Zepto (MSPM0) over UART.
- `dfu`: Flash devices over USB DFU.
- `static-hidraw`: Statically link hidraw.

Optional:

- `bcf_msp430`: Flash MSP430 in BeagleConnect Freedom, which acts as the USB to UART bridge.
- `zepto_i2c`: Flash Zepto (MSPM0) over I2C. Linux only.
- `pb2_mspm0`: Flash PocketBeagle 2 MSPM0. Needs root permissions.
- `shared-hidraw`: Dynamically link hidraw instead.
