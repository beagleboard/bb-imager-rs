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
## variable: NO_BUILD: Do not build any packages. Useful for cross builds in CI.
NO_BUILD ?= 0

ifeq (${VERBOSE}, 1)
	_RUST_ARGS+=--verbose
	_CARGO_PACKAGER_ARGS+=--verbose
endif

_RUST_ARGS_CLI_GUI = ${_RUST_ARGS}
_RUST_ARGS_SERVICE = ${_RUST_ARGS}

ifeq (${PB2_MSPM0}, 1)
	_RUST_ARGS_CLI_GUI+=-F pb2_mspm0
endif
ifeq (${BCF_CC1352P7}, 1)
	_RUST_ARGS_CLI_GUI+=-F bcf_cc1352p7
endif
ifeq (${BCF_MSP430}, 1)
	_RUST_ARGS_CLI_GUI+=-F bcf_msp430
endif

## housekeeping: help: Display this help message
.PHONY: help
help:
	@python scripts/make_help.py Makefile

## housekeeping: clean: Clean the project files
.PHONY: clean
clean:
	@echo "Cleaning the project..."
	$(CARGO_PATH) clean
	rm -rf target
	rm -rf bb-imager-gui/dist
	rm -rf bb-imager-cli/dist
	rm -rf bb-imager-service/dist

## housekeeping: check: Run code quality checks.
.PHONY: check
check:
	@echo "Running clippy checks"
	$(CARGO_PATH) clippy --all-targets --all-features --no-deps --workspace ${_RUST_ARGS}

## housekeeping: test: Run tests on workspace
.PHONY: test
test:
	@echo "Run workspace tests"
	$(CARGO_PATH) test --workspace --all-features ${_RUST_ARGS}

## build: build-gui: Build GUI. Target platform can be changed using TARGET env variable.
.PHONY: build-gui
build-gui:
ifeq (${NO_BUILD}, 1)
	@echo "Skip Building GUI"
else
	@echo "Building GUI"
	$(RUST_BUILDER) build -r -p bb-imager-gui --target ${TARGET} ${_RUST_ARGS_CLI_GUI}
endif

## run: run-gui: Run GUI for quick testing on host.
.PHONY: run-gui
run-gui:
	@echo "Running GUI"
	$(CARGO_PATH) run -p bb-imager-gui

## build: build-cli: Build CLI. Target platform can be changed using TARGET env variable.
.PHONY: build-cli
build-cli:
ifeq (${NO_BUILD}, 1)
	@echo "Skip Building CLI"
else
	@echo "Building CLI"
	$(RUST_BUILDER) build -r -p bb-imager-cli --target ${TARGET} ${_RUST_ARGS_CLI_GUI}
endif

## run: run-cli: Run CLI for quick testing on host.
.PHONY: run-cli
run-cli:
	@echo "Running CLI"
	$(CARGO_PATH) run -p bb-imager-cli

## build: build-service: Build BeagleBoard Service. Target platform can be changed using TARGET env variable.
.PHONY: build-service
build-service:
ifeq (${NO_BUILD}, 1)
	@echo "Skip Building SERVICE"
else
	@echo "Building Service"
	$(RUST_BUILDER) build -r -p bb-imager-service --target ${TARGET} ${_RUST_ARGS_SERVICE}
endif


## build: build-cli-manpage: Build manpage for CLI.
.PHONY: build-cli-manpage
build-cli-manpage:
	@echo "Generate CLI Manpages"
	rm -rf bb-imager-cli/dist/.target/man
	mkdir -p bb-imager-cli/dist/.target/man
	$(CARGO_PATH) xtask ${_RUST_ARGS_CLI_GUI} cli-man bb-imager-cli/dist/.target/man/
	gzip bb-imager-cli/dist/.target/man/*


## build: build-cli-shell-comp: Build shell completion for CLI.
.PHONY: build-cli-shell-comp
build-cli-shell-comp:
	@echo "Generate CLI completion"
	rm -rf bb-imager-cli/dist/.target/shell-comp
	mkdir -p bb-imager-cli/dist/.target/shell-comp
	$(CARGO_PATH) xtask ${_RUST_ARGS_CLI_GUI} cli-shell-complete zsh bb-imager-cli/dist/.target/shell-comp
	$(CARGO_PATH) xtask ${_RUST_ARGS_CLI_GUI} cli-shell-complete bash bb-imager-cli/dist/.target/shell-comp

## package: package-gui-linux-appimage: Build AppImage package for GUI.
.PHONY: package-gui-linux-appimage
package-gui-linux-appimage: build-gui
	@echo "Packaging GUI as appimage"
	$(CARGO_PATH) packager -p bb-imager-gui --target ${TARGET} -f appimage ${_CARGO_PACKAGER_ARGS}

## package: package-gui-linux-deb: Build Debian package for GUI
.PHONY: package-gui-linux-deb
package-gui-linux-deb: build-gui
	@echo "Packaging GUI as deb"
	$(CARGO_PATH) packager -p bb-imager-gui --target ${TARGET} -f deb ${_CARGO_PACKAGER_ARGS}

## package: package-cli-linux-deb: Build Debian package for CLI
.PHONY: package-cli-linux-deb
package-cli-linux-deb: build-cli build-cli-manpage build-cli-shell-comp
	@echo "Packaging CLI as deb"
	$(CARGO_PATH) packager -p bb-imager-cli --target ${TARGET} -f deb ${_CARGO_PACKAGER_ARGS}

## package: package-service-linux-deb: Build Debian package for Service
.PHONY: package-service-linux-deb
package-service-linux-deb: build-service
	@echo "Packaging Service as deb"
	$(CARGO_PATH) packager -p bb-imager-service --target ${TARGET} -f deb ${_CARGO_PACKAGER_ARGS}

## package: package-gui-linux-targz: Build generic linux package for GUI
.PHONY: package-gui-linux-targz
package-gui-linux-targz: build-gui
	@echo "Packaging GUI as generic linux tar.gz"
	$(CARGO_PATH) packager -p bb-imager-gui --target ${TARGET} -f pacman ${_CARGO_PACKAGER_ARGS}
	rm bb-imager-gui/dist/PKGBUILD

## package: package-cli-linux-targz: Build generic linux package for CLI
.PHONY: package-cli-linux-targz
package-cli-linux-targz: build-cli build-cli-manpage build-cli-shell-comp
	@echo "Packaging CLI as generic linux tar.gz"
	$(CARGO_PATH) packager -p bb-imager-cli --target ${TARGET} -f pacman ${_CARGO_PACKAGER_ARGS}
	rm bb-imager-cli/dist/PKGBUILD

## package: package-service-linux-targz: Build generic Linux package for Service
.PHONY: package-service-linux-targz
package-service-linux-targz: build-service
	@echo "Packaging Service as generic linux tar.gz"
	$(CARGO_PATH) packager -p bb-imager-service --target ${TARGET} -f pacman ${_CARGO_PACKAGER_ARGS}
	rm bb-imager-service/dist/PKGBUILD

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
	$(CARGO_PATH) packager -p bb-imager-gui --target ${TARGET} -f wix ${_CARGO_PACKAGER_ARGS}

## package: package-gui-macos-dmg: Build MacOS DMG package for GUI
.PHONY: package-gui-macos-dmg
package-gui-macos-dmg: build-gui
	@echo "Packaging GUI as DMG"
	$(CARGO_PATH) packager -p bb-imager-gui --target ${TARGET} -f dmg ${_CARGO_PACKAGER_ARGS}

## setup: setup-debian-deps: Install debian dependencies for building. For creating packages, also run setup-packaging-deps
.PHONY: setup-debian-deps
setup-debian-deps:
	@echo "Installing Debian dependencies"
	sudo apt-get update -y
	sudo apt-get install -y --no-install-recommends libudev-dev

## setup: setup-fedora-deps: Install Fedora Linux dependencies for building. For creating packages, also run setup-packaging-deps
.PHONY: setup-fedora-deps
setup-fedora-deps:
	@echo "Installing Fedora dependencies"
	sudo dnf install -y openssl-devel systemd-devel

## setup: setup-packaging-deps: Install dependencies for generting packages.
.PHONY: setup-packaging-deps
setup-packaging-deps:
	@echo "Installing dependencies required for packaging"
	$(CARGO_PATH) install cargo-packager --locked --git https://github.com/Ayush1325/cargo-packager.git --branch bb-imager


## housekeeping: package-rename: Replace package version with `_alpha_`. Intended for use in CI.
.PHONY: package-rename-alpha
package-rename:
	for pkg in gui cli service; do \
		if [ -d bb-imager-$$pkg/dist ]; then \
			for file in bb-imager-$$pkg/dist/*; do \
				mv "$$file" "$$(echo "$$file" | sed -E 's/_[0-9]+\.[0-9]+\.[0-9]+_/_alpha_/')"; \
			done \
		fi \
	done
