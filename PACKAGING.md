# Packaging

## Task system

- This project uses `make` as the task runner. For most people, running `make` should be enough get a grasp regarding how to build and run the project.

## Building

The repo contains a `Makefile` to help create the different supported packages.

## Packaging tools

This project uses [cargo-packager](https://crates.io/crates/cargo-packager) to build the target packages. Install it first via:

```sh
make setup-packaging-deps
```

## Deb Package

- CLI

```
make package-cli-linux-deb
```

- GUI

```
make package-gui-linux-deb
```

- Service

```
make package-service-linux-deb
```

The target platform can be specified with `TARGET` environment variable. Defaults to host target. Currently, the following targets have been tested:
- x86_64-unknown-linux-gnu
- aarch64-unknown-linux-gnu
- armv7-unknown-linux-gnueabihf

For different host/target pair, see [Cross Compilation](#cross-compilation)

## AppImage

```
make package-gui-linux-appimage
```

The target platform can be specified with `TARGET` environment variable. Defaults to host target. Currently, the following targets have been tested:
- x86_64-unknown-linux-gnu
- aarch64-unknown-linux-gnu

For different host/target pair, see [Cross Compilation](#cross-compilation)

## Linux Generic

Just a tarball of everything. Useful for creating packages that need to be maintained out of tree.

- GUI

```
make package-gui-linux-targz
```

- CLI

```
make package-cli-linux-targz
```

- Service

```
make package-service-linux-targz
```

The target platform can be specified with `TARGET` environment variable. Defaults to host target. Currently, the following targets have been tested:
- x86_64-unknown-linux-gnu
- aarch64-unknown-linux-gnu
- armv7-unknown-linux-gnueabihf

For different host/target pair, see [Cross Compilation](#cross-compilation)

## Windows Standalone Executable

- GUI

```
make package-gui-windows-portable
```

The target platform can be specified with `TARGET` environment variable. Defaults to host target. Currently, the following targets have been tested:
- x86_64-pc-windows-gnu
- x86_64-pc-windows-msvc
- aarch64-pc-windows-msvc

For different host/target pair, see [Cross Compilation](#cross-compilation)

## Windows Installer

- GUI

```
make package-gui-windows-wix
```

The target platform can be specified with `TARGET` environment variable. Defaults to host target. Currently, the following targets have been tested:
- x86_64-pc-windows-msvc

For different host/target pair, see [Cross Compilation](#cross-compilation)

## MacOS DMG

Build the package.

```
make package-gui-macos-dmg
```

The target platform can be specified with `TARGET` environment variable. Defaults to host target. Currently, the following targets have been tested:
- x86_64-apple-darwin
- aarch64-apple-darwin

MacOS Package cannot be built on a non-MacOS host.

# Cross Compilation

Cross Compilation for linux is supported using [cross](https://github.com/cross-rs/cross). It is important to note that the git version of `cross` is required right now.

```
cargo install cross --git https://github.com/cross-rs/cross
RUST_BUILDER=$(which cross) make {recipe}
```
