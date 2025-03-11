# Packaging

## Task system

- This project uses [just](https://just.systems/) as the task runner. For most people, running `just` should be enough get a grasp regarding how to build and run the project.

## Building

The repo contains a `Makefile` to help create the different supported packages.

## Deb Package

1. The project uses [cargo-deb](https://crates.io/crates/cargo-deb) to build the Debian package. It can be installed with a just recipe

```
just setup-deb
```

2. Build the packages

- CLI

```
just package-cli-linux-deb {target}
```

- GUI

```
just package-gui-linux-deb {target}
```

- Service

```
just package-service-linux-deb {target}
```

Where `target` is the platform you are building for. Currently, the following targets have been tested:
- x86_64-unknown-linux-gnu
- aarch64-unknown-linux-gnu
- armv7-unknown-linux-gnueabihf

For different host/target pair, see [Cross Compilation](#cross-compilation)

## AppImage

1. The project uses [appimagetool](https://github.com/AppImage/appimagetool) for building the AppImage.

```
just setup-appimage
```

2. Build the package

```
just package-gui-linux-appimage {target}
```

Where `target` is the platform you are building for. Currently, the following targets have been tested:
- x86_64-unknown-linux-gnu
- aarch64-unknown-linux-gnu
- armv7-unknown-linux-gnueabihf

For different host/target pair, see [Cross Compilation](#cross-compilation)

## Linux Generic

Just a tarball of everything. Useful for creating packages that need to be maintained out of tree.

- CLI

```
just package-cli-linux-xz {target}
```

- Service

```
just package-service-linux-xz {target}
```

Where `target` is the platform you are building for. Currently, the following targets have been tested:
- x86_64-unknown-linux-gnu
- aarch64-unknown-linux-gnu
- armv7-unknown-linux-gnueabihf

For different host/target pair, see [Cross Compilation](#cross-compilation)

## Windows Standalone Executable

- CLI

```
just package-cli-windows-zip {target}
```

- GUI

```
just package-gui-windows-zip {target}
```

Where `target` is the platform you are building for. Currently, the following targets have been tested:
- x86_64-pc-windows-gnu

For different host/target pair, see [Cross Compilation](#cross-compilation)

## MacOS DMG

1. The project uses [create-dmg](https://github.com/create-dmg/create-dmg) to build DMG package for MacOS

```
just setup-dmg
```

2. Build the package.

```
just package-gui-darwin-dmg {target}
```

Where `target` is the platform you are building for. Currently, the following targets have been tested:
- x86_64-apple-darwin
- aarch64-apple-darwin
- universal-apple-darwin

MacOS Package cannot be built on a non-MacOS host.

## MacOS Generic

Zipped package for CLI. Useful for creating out of tree packages. Only supported for CLI.

```
just package-cli-darwin-zip {target}
```

Where `target` is the platform you are building for. Currently, the following targets have been tested:
- x86_64-apple-darwin
- aarch64-apple-darwin
- universal-apple-darwin

MacOS Package cannot be built on a non-MacOS host.

# Cross Compilation

Cross Compilation for linux is supported using [cross](https://github.com/cross-rs/cross). It is important to note that the git version of `cross` is required right now.

```
cargo install cross --git https://github.com/cross-rs/cross
RUST_BUILDER=$(which cross) just {recipe} {target}
```
