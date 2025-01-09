CARGO_PATH = $(shell which cargo)
RUST_BUILDER ?= $(CARGO_PATH)
APPIMAGETOOL ?= $(shell which appimagetool)
RUST_BUILDER_NAME = $(lastword $(subst /,  , $(RUST_BUILDER)))
CROSS_UTIL ?= $(shell which cross-util)

VERSION ?= $(shell grep 'version =' Cargo.toml | sed 's/version = "\(.*\)"/\1/')

RELEASE_DIR ?= $(CURDIR)/release
RELEASE_DIR_LINUX ?= ${RELEASE_DIR}/linux
RELEASE_DIR_WINDOWS ?= ${RELEASE_DIR}/windows
RELEASE_DIR_DARWIN ?= ${RELEASE_DIR}/darwin

GUI_ASSETS = $(CURDIR)/bb-imager-gui/assets
GUI_ASSETS_LINUX = ${GUI_ASSETS}/packages/linux
GUI_ASSETS_DARWIN = ${GUI_ASSETS}/packages/darwin

# Includes
include bb-imager-gui/Makefile
include bb-imager-cli/Makefile
include scripts/*.mk

clean:
	$(CARGO_PATH) clean
	rm -rf release

release-linux-%: package-cli-linux-xz-% package-cli-linux-deb-% package-gui-linux-appimage-% package-gui-linux-deb-%;

release-darwin-%: package-cli-darwin-zip-% package-gui-darwin-dmg-%;

release-windows-%: package-cli-windows-zip-% package-gui-windows-zip-%;

upload-artifacts: upload-artifact-linux upload-artifact-windows upload-artifact-darwin;
