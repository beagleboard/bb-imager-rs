# Packaging

## Dependencies

The following dependencies are only required to build a package locally. It can be skipped during general development.

### Deb Packaging

The project uses [cargo-deb](https://crates.io/crates/cargo-deb) to build the Debian package.

```
cargo install cargo-deb
```

### AppImage Packaging

- The project uses [appimagetool](https://github.com/AppImage/appimagetool) for building the AppImage.

```
wget https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$(uname -m).AppImage -O $HOME/.local/bin/appimagetool
chmod +x $HOME/.local/bin/appimagetool
```

### DMG Packaging

- The project uses [create-dmg](https://github.com/create-dmg/create-dmg) to build DMG package for MacOS

```
brew install create-dmg
```

## Building

The repo contains a `Makefile` to help create the different supported packages.

## Deb Package

Ensure [dependencies](#deb-packaging) are installed.

- CLI

```
make package-cli-linux-deb-{target}
```

- GUI

```
make package-gui-linux-deb-{target}
```

- Service

```
make package-service-linux-deb-{target}
```

Where `target` is the platform you are building for. Currently, the following targets have been tested:
- x86_64-unknown-linux-gnu
- aarch64-unknown-linux-gnu
- armv7-unknown-linux-gnueabihf

For different host/target pair, see [Cross Compilation](#cross-compilation)

## AppImage

Ensure [dependencies](#appimage-dependencies) are installed. AppImage is only supported for the GUI.

```
make package-gui-linux-appimage-{target}
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
make package-cli-linux-xz-{target}
```

- Service

```
make package-service-linux-xz-{target}
```

Where `target` is the platform you are building for. Currently, the following targets have been tested:
- x86_64-unknown-linux-gnu
- aarch64-unknown-linux-gnu
- armv7-unknown-linux-gnueabihf

For different host/target pair, see [Cross Compilation](#cross-compilation)

## Windows Standalone Executable

- CLI

```
make package-cli-windows-zip-{target}
```

- GUI

```
make package-gui-windows-zip-{target}
```

Where `target` is the platform you are building for. Currently, the following targets have been tested:
- x86_64-pc-windows-gnu

For different host/target pair, see [Cross Compilation](#cross-compilation)

## MacOS DMG

Ensure [dependencies](#dmg-dependencies) are installed. DMG is only supported for the GUI.

```
make package-gui-darwin-dmg-{target}
```

Where `target` is the platform you are building for. Currently, the following targets have been tested:
- x86_64-apple-darwin
- aarch64-apple-darwin
- universal-apple-darwin

MacOS Package cannot be built on a non-MacOS host.

## MacOS Generic

Zipped package for CLI. Useful for creating out of tree packages. Only supported for CLI.

```
make package-cli-darwin-zip-{target}
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
RUST_BUILDER=$(which cross) make {target}
```
