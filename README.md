# BB Imager Rust

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
    - [ ] BeagleConnect Freedom MSP430
    - [ ] BeaglePlay CC1352
- [ ] Support flash time configuration (ssh, wifi, etc)
- [ ] Remote `config.json` file

# Run

```shell
cargo run --package gui --release
```

# Configuration

The boards and images are configured using a `config.json` file. This file will typically reside in a remote server. It is quite similar to the one used in `bb-imager` with slight modifications to allow use with non-linux targets along with more verfication.

See [config.json](config.json) for example.
