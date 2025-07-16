# BeagleBoard Imager Rust

BeagleBoard Imaging Utility, a streamlined tool for creating, flashing, and managing OS images for BeagleBoard devices.

# Contributing

Please see [Contributing.md](CONTRIBUTING.md)

# Packaging

Please see [Packaging.md](PACKAGING.md)

# Configuration

The boards and images are configured using a `config.json` file. This file will typically reside in a remote server. It is quite similar to the one used in `bb-imager` with slight modifications to allow use with non-linux targets along with more verfication.

See [config.json](config.json) for example.

# GUI

![BBImager Home Screen](assets/screenshots/home.webp)
![BBImager Configuration Screen](assets/screenshots/config.webp)
![BBImager Flashing Screen](assets/screenshots/flash.webp)

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

Usage: bb-imager-cli flash sd [OPTIONS] <IMG> <DST>

Arguments:
  <IMG>  Local path to image file. Can be compressed (xz) or extracted file
  <DST>  The destination device (e.g., `/dev/sdX` or specific device identifiers)

Options:
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

## Flashing image

```shell
❯ bb-imager-cli flash --quiet bcf $IMG_PATH /dev/ttyACM0
```

# Creating Issues

While creating new issues for bugs, please attach logs from the application. Log files are created automatically by the GUI from v0.0.12.

On Linux the log file is stored in cache directory (eg. `$HOME/.cache/org.beagleboard.imagingutility.log`) and on Windows, it seems be at `C:\Users\ayush\AppData\Local\beagleboard\imagingutility\org.beagleboard.imagingutility.log`. If anyone can find out MacOS path, feel free to add here.
