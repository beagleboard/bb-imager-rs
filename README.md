# BeagleBoard Imager Rust

BeagleBoard Imaging Utility, a streamlined tool for creating, flashing, and managing OS images for BeagleBoard devices.

![BBImager Home Screen](./assets/screenshots/1_board_selection.webp)

# Documentation

Full documentation: **https://beagleboard.github.io/bb-imager-rs/**

- [Install the GUI](docs/modules/ROOT/pages/install-gui.adoc) and [flash your first board](docs/modules/ROOT/pages/quickstart-gui.adoc)
- [Install the CLI](docs/modules/ROOT/pages/install-cli.adoc) and [use it](docs/modules/ROOT/pages/cli-usage.adoc)
- [Supported hardware](docs/modules/ROOT/pages/supported-hardware.adoc)
- [Troubleshooting and reporting issues](docs/modules/ROOT/pages/troubleshooting.adoc)
- [Development guide](docs/modules/ROOT/pages/development.adoc), [Contributing](./CONTRIBUTING.adoc), [Packaging](./PACKAGING.adoc)

# Configuration

The boards and images are configured using a `config.json` file. This file will typically reside in a remote server. It is quite similar to the one used in `bb-imager` with slight modifications to allow use with non-linux targets along with more verfication.

See [config.json](config.json) for example.
