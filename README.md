# BeagleBoard Imager Rust

A Rust rewrite of [bb-imager](https://openbeagle.org/beagleboard/bb-imager) with support for flashing boards not using Linux.

# Goals
- [ ] Cross Platform
    - [x] Linux
    - [x] Windows
    - [ ] Macos
    - [ ] Web
- [ ] Supported Flashers
    - [x] Generic Linux (BeaglePlay, Beagle AI64, etc)
    - [x] BeagleConnect Freedom
    - [x] BeagleConnect Freedom MSP430
    - [ ] BeaglePlay CC1352
- [x] Support flash time configuration (ssh, wifi, etc)
- [ ] Remote `config.json` file

# Run

```shell
cargo run --package bb-imager-gui --release
```

# Configuration

The boards and images are configured using a `config.json` file. This file will typically reside in a remote server. It is quite similar to the one used in `bb-imager` with slight modifications to allow use with non-linux targets along with more verfication.

See [config.json](config.json) for example.

# GUI

![BBImager Home Screen](screenshots/home.png)
![BBImager Configuration Screen](screenshots/config.png)
![BBImager Flashing Screen](screenshots/flash.png)

# CLI

## Home Help

```shell
❯ bb-imager-cli --help
A streamlined tool for creating, flashing, and managing OS images for BeagleBoard devices.

Usage: bb-imager-cli <COMMAND>

Commands:
  flash                Command to flash an image to a specific destination
  list-destinations    Command to list available destinations for flashing based on the selected target
  format               Command to format SD Card
  generate-completion  Command to generate shell completion
  help                 Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Flashing SD Card Help

```shell
❯ bb-imager-cli flash sd --help
Flash an SD card with customizable settings for BeagleBoard devices

Usage: bb-imager-cli flash sd [OPTIONS] <--img-local <IMG_LOCAL>|--img-remote <IMG_REMOTE>> <DST>

Arguments:
  <DST>  The destination device (e.g., `/dev/sdX` or specific device identifiers)

Options:
      --img-local <IMG_LOCAL>          Path to the image file to flash. Supports both raw and compressed (e.g., xz) formats
      --img-remote <IMG_REMOTE>        URL to remote image file to flash. Supports both raw and compressed (e.g., xz) formats
      --img-sha256 <IMG_SHA256>        Checksum for remote image
      --no-verify                      Disable checksum verification post-flash
      --hostname <HOSTNAME>            Set a custom hostname for the device (e.g., "beaglebone")
      --timezone <TIMEZONE>            Set the timezone for the device (e.g., "America/New_York")
      --keymap <KEYMAP>                Set the keyboard layout/keymap (e.g., "us" for the US layout)
      --user-name <USER_NAME>          Set a username for the default user. Requires `user_password`.
                                       Required to enter GUI session due to regulatory requirements.
      --user-password <USER_PASSWORD>  Set a password for the default user. Requires `user_name`.
                                       Required to enter GUI session due to regulatory requirements.
      --wifi-ssid <WIFI_SSID>          Configure a Wi-Fi SSID for network access. Requires `wifi_password`
      --wifi-password <WIFI_PASSWORD>  Set the password for the specified Wi-Fi SSID. Requires `wifi_ssid`
  -h, --help                           Print help
```

## Flashing Remote image

```shell
❯ bb-imager-cli flash bcf --image-remote $IMG_URL --image-sha256 $IMG_SHA256 /dev/ttyACM0
[1] Preparing
[2] Verifying    [█████████████████████████████████████████████████████████████████████████████████████████████████████████████] [100 %]
[3] Flashing     [█████████████████████████████████████████████████████████████████████████████████████████████████████████████] [100 %]
[4] Verifying
```

## Flashing Local image

```shell
❯ bb-imager-cli flash --quiet bcf $DESTINATION --img-local $IMG_PATH /dev/ttyACM0
```
