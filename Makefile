# Run 'make help' to see guidance on usage of this Makefile

_HOST_TARGET = $(shell rustc -vV | awk '/^host/ { print $$2 }')
_RUST_ARGS = --locked
_CARGO_PACKAGER_ARGS = -r
_TARGET_ARCH = $(shell echo ${TARGET} | cut -d'-' -f1)

## variable: CARGO_PATH: Path to cargo binary
CARGO_PATH ?= $(shell which cargo)
## variable: RUST_BUILDER: The Rust builder to use. Possble choices - cargo (default), cross.
RUST_BUILDER ?= $(CARGO_PATH)
## variable: VERSION: Release versions for bb-imager-cli and bb-imager-gui
VERSION ?= $(shell grep 'version =' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
## variable: PB2_MSPM0: Enable support for PocketBeagle 2 MSPM0
PB2_MSPM0 ?= 0
## variable: BCF_CC1352P7: Enable support for BeagleConnect Freedom CC1352P7
BCF_CC1352P7 ?= 0
## variable: BCF_MSP430: Enable support for BeagleConnect Freedom MSP430
BCF_MSP430 ?= 0
## variable: VERBOSE: Enable verbose logging. Useful in CI
VERBOSE ?= 0
## variable: TARGET: Package Target platform. Defaults to Host target.
TARGET ?= ${_HOST_TARGET}

ifeq (${VERBOSE}, 1)
	_RUST_ARGS+=--verbose
	_CARGO_PACKAGER_ARGS+=--verbose
endif

ifeq (${PB2_MSPM0}, 1)
	_RUST_ARGS+=-F pb2_mspm0
endif
ifeq (${BCF_CC1352P7}, 1)
	_RUST_ARGS+=-F bcf_cc1352p7
endif
ifeq (${BCF_MSP430}, 1)
	_RUST_ARGS+=-F bcf_msp430
endif

## default: help: Display this help message
.PHONY: help
help:
	@python scripts/make_help.py Makefile

## default: clean: Clean the project files
.PHONY: clean
clean:
	@echo "Cleaning the project..."
	${CARGO_PATH} clean
	rm -rf target
	rm -rf bb-imager-gui/dist
	rm -rf bb-imager-cli/dist

## build: build-gui: Build GUI. Target platform can be changed using TARGET env variable.
.PHONY: build-gui
build-gui:
	@echo "Building GUI"
	${RUST_BUILDER} build -r -p bb-imager-gui --target ${TARGET} ${_RUST_ARGS}

## package: package-gui-linux-appimage: Build AppImage package for GUI.
.PHONY: package-gui-linux-appimage
package-gui-linux-appimage: build-gui
	@echo "Packaging GUI as appimage"
	${CARGO_PATH} packager -p bb-imager-gui --target ${TARGET} -f appimage ${_CARGO_PACKAGER_ARGS}

## package: package-gui-linux-deb: Build Debian package for GUI
.PHONY: package-gui-linux-deb
package-gui-linux-deb: build-gui
	@echo "Packaging GUI as deb"
	${CARGO_PATH} packager -p bb-imager-gui --target ${TARGET} -f deb ${_CARGO_PACKAGER_ARGS}

## package: package-gui-linux-targz: Build generic linux package for GUI
.PHONY: package-gui-linux-targz
package-gui-linux-targz: build-gui
	@echo "Packaging GUI as deb"
	${CARGO_PATH} packager -p bb-imager-gui --target ${TARGET} -f pacman ${_CARGO_PACKAGER_ARGS}
	rm bb-imager-gui/dist/PKGBUILD

## package: package-gui-windows-portable: Build portable Windows exe package for GUI
.PHONY: package-gui-windows-portable
package-gui-windows-portable: build-gui
	@echo "Packaging GUI as portable Windows exe"
	mkdir -p bb-imager-gui/dist
	cp target/${TARGET}/release/bb-imager-gui.exe bb-imager-gui/dist/bb-imager-gui_${VERSION}_${_TARGET_ARCH}.exe

## package: package-gui-windows-wix: Build Windows installer for GUI
.PHONY: package-gui-windows-wix
package-gui-windows-wix: build-gui
	@echo "Packaging GUI as windows installer"
	${CARGO_PATH} packager -p bb-imager-gui --target ${TARGET} -f wix ${_CARGO_PACKAGER_ARGS}

## package: package-gui-macos-dmg: Build MacOS DMG package for GUI
.PHONY: package-gui-macos-dmg
package-gui-macos-dmg: build-gui
	@echo "Packaging GUI as deb"
	${CARGO_PATH} packager -p bb-imager-gui --target ${TARGET} -f dmg ${_CARGO_PACKAGER_ARGS}

## setup: setup-debian-deps: Install debian dependencies for building. For creating packages, also run setup-packaging-deps
.PHONY: setup-debian-deps
setup-debian-deps:
	@echo "Installing dependencies"
	sudo apt-get update -y
	sudo apt-get install -y --no-install-recommends libudev-dev

## setup: setup-packaging-deps: Install dependencies for generting packages.
.PHONY: setup-packaging-deps
setup-packaging-deps:
	@echo "Installing dependencies required for packaging"
	cargo install cargo-packager --locked
