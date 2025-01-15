CARGO_PATH = $(shell which cargo)
RUST_BUILDER ?= $(CARGO_PATH)
APPIMAGETOOL ?= $(shell which appimagetool)
RUST_BUILDER_NAME = $(lastword $(subst /,  , $(RUST_BUILDER)))
CROSS_UTIL ?= $(shell which cross-util)

# Features related stuff
RUST_FEATURE_ARGS =
PB2_MSPM0 ?=

VERSION ?= $(shell grep 'version =' Cargo.toml | sed 's/version = "\(.*\)"/\1/')

RELEASE_DIR ?= $(CURDIR)/release
RELEASE_DIR_LINUX ?= ${RELEASE_DIR}/linux
RELEASE_DIR_WINDOWS ?= ${RELEASE_DIR}/windows
RELEASE_DIR_DARWIN ?= ${RELEASE_DIR}/darwin

GUI_ASSETS = $(CURDIR)/bb-imager-gui/assets
GUI_ASSETS_LINUX = ${GUI_ASSETS}/packages/linux
GUI_ASSETS_DARWIN = ${GUI_ASSETS}/packages/darwin

SERVICE_ASSETS = $(CURDIR)/bb-imager-service/assets

# Includes
include bb-imager-gui/Makefile
include bb-imager-cli/Makefile
include bb-imager-service/Makefile
include scripts/*.mk

clean:
	$(CARGO_PATH) clean
	rm -rf release

release-linux-%: package-cli-linux-xz-% package-cli-linux-deb-% package-gui-linux-appimage-% package-gui-linux-deb-% $(if $(PB2_MSPM0), package-service-linux-deb-% package-service-linux-xz-%);

release-darwin-%: package-cli-darwin-zip-% package-gui-darwin-dmg-%;

release-windows-%: package-cli-windows-zip-% package-gui-windows-zip-%;

upload-artifacts: upload-artifact-linux upload-artifact-windows upload-artifact-darwin;

checks-clippy-%:
	$(info "Running clippy checks for $*")
	$(CARGO_PATH) clippy -p $* --all-targets --all-features --no-deps

checks: checks-clippy-bb-imager checks-clippy-bb-imager-cli checks-clippy-bb-imager-gui checks-clippy-bb-imager-service
