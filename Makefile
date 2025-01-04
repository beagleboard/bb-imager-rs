RUST_BUILDER ?= $(shell which cargo)
APPIMAGETOOL ?= $(shell which appimagetool)

VERSION ?= $(shell grep 'version =' Cargo.toml | sed 's/version = "\(.*\)"/\1/')

RELEASE_DIR ?= $(CURDIR)/release
RELEASE_DIR_LINUX ?= ${RELEASE_DIR}/linux
RELEASE_DIR_WINDOWS ?= ${RELEASE_DIR}/windows
RELEASE_DIR_DARWIN ?= ${RELEASE_DIR}/darwin

GUI_ASSETS = $(CURDIR)/gui/assets
GUI_ASSETS_LINUX = ${GUI_ASSETS}/packages/linux
GUI_ASSETS_DARWIN = ${GUI_ASSETS}/packages/darwin

# Map Rust targets with Appimage Arch
APPIMAGE_ARCH_x86_64-unknown-linux-gnu = x86_64
APPIMAGE_ARCH_aarch64-unknown-linux-gnu = aarch64
APPIMAGE_ARCH_armv7-unknown-linux-gnueabihf = armhf

# Includes
include gui/Makefile
include cli/Makefile
include scripts/*.mk

clean:
	cargo clean
	rm -rf release

release-linux-%: package-cli-linux-xz-% package-cli-linux-deb-% package-gui-linux-appimage-% package-gui-linux-deb-%;

release-darwin-%: package-cli-darwin-zip-% package-gui-darwin-dmg-%;

release-windows-%: package-cli-windows-zip-% package-gui-windows-zip-%;

upload-artifacts: upload-artifact-linux upload-artifact-windows upload-artifact-darwin;
