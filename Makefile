# Run 'make help' to see guidance on usage of this Makefile

_HOST_TARGET = $(shell rustc --print host-tuple)
_CARGO_TOML_VERSION = $(shell grep 'version =' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
_DATE = $(shell date +%F)
_RUST_ARGS_BASE = --locked
_RUST_ARGS = ${_RUST_ARGS_BASE} -r -F bcf_cc1352p7,bcf_msp430,zepto_uart
_RUST_ARGS_CLI = ${_RUST_ARGS} -F dfu
_RUST_ARGS_GUI = ${_RUST_ARGS}
_PACKAGER_ARGS = -r -vvv --verbose

_CLI_BIN = target/${TARGET}/release/bb-imager-cli
_CLI_COMP_BASH = bb-imager-cli/dist/.target/shell-comp/bb-imager-cli.bash
_CLI_COMP_ZSH = bb-imager-cli/dist/.target/shell-comp/_bb-imager-cli
_CLI_MAN = bb-imager-cli/dist/.target/man/bb-imager-cli.1.gz

_GUI_BIN = target/${TARGET}/release/bb-imager-gui
_GUI_PORTABLE_EXE = bb-imager-gui/dist/bb-imager-gui_$(VERSION)_$(word 1,$(subst -, ,$(TARGET))).exe

## variable: GUI_NAME: Change name for GUI related files.
GUI_NAME ?= BeagleBoardImager
## variable: CARGO_PATH: Path to cargo binary
CARGO_PATH ?= $(shell which cargo)
## variable: RUST_BUILDER: The Rust builder to use. Possble choices - cargo (default), cross.
RUST_BUILDER ?= $(CARGO_PATH)
## variable: VERSION: Release versions for bb-imager-cli and bb-imager-gui
VERSION ?= $(_CARGO_TOML_VERSION)
## variable: NO_BUILD: Do not build any packages. Useful for cross builds in CI.
NO_BUILD ?= 0
## variable: VERBOSE: Enable verbose logging.
VERBOSE ?= 0
## variable: OFFLINE: Should be used when building flatpak.
OFFLINE ?= 0
## variable: PREFIX: Install Prefix
PREFIX ?= /usr/local
## variable: BINDIR: Directory to install binary
BINDIR ?= $(PREFIX)/bin
## variable: MANDIR: Directory to install manpages
MANDIR ?= $(PREFIX)/share/man
## variable: BASH_COMPLETIONDIR: Directory to install bash completions
BASH_COMPLETIONDIR ?= $(PREFIX)/share/bash-completion/completions
## variable: ZSH_COMPLETIONDIR: Directory to install zsh completions
ZSH_COMPLETIONDIR ?= $(PREFIX)/share/zsh/site-functions
## variable: UDEV_RULESDIR: Directory to install udev rules to
UDEV_RULESDIR ?= /etc/udev/rules.d/
## variable: ICONS_DIR: Directory to install icons to.
ICONS_DIR ?= $(PREFIX)/share/icons
## variable: DESKTOP_DIR: Directory to install desktop entry to
DESKTOP_DIR ?= $(PREFIX)/share/applications
## variable: METAINFO_DIR: Directory to install metainfo file.
METAINFO_DIR ?= $(PREFIX)/share/metainfo
## variable: TARGET: Compilation Target. Default = host
TARGET ?= $(_HOST_TARGET)
## variable: PB2_MSPM0: Enable pb2_mspm0 feature. Only used in CLI.
PB2_MSPM0 ?= 0
## variable: ZEPTO_I2C: Enable zepto_i2c feature. Only supported in GUI.
ZEPTO_I2C ?= $(if $(findstring linux,$(TARGET)),1)
## variable: SYSTEM_DEPS: Use system dependencies. Mainly for linux.
SYSTEM_DEPS ?= 0
## variable: UPDATER: Enable updater feature in GUI.
UPDATER ?= 0

# Allow skipping build step
ifeq ($(NO_BUILD),1)
RUST_BUILD = @true
else
RUST_BUILD = $(RUST_BUILDER) build
endif

# Add verbose flag is needed
ifeq ($(VERBOSE),1)
	_RUST_ARGS_BASE += --verbose
endif

# Add zepto_i2c feature
ifeq ($(ZEPTO_I2C),1)
	_RUST_ARGS += -F zepto_i2c
endif

# Add system-deps feature
ifeq ($(SYSTEM_DEPS),1)
	_RUST_ARGS_GUI += --no-default-features -F system-deps
endif

# Add offline flag is needed
ifeq ($(OFFLINE),1)
	_RUST_ARGS_BASE += --offline
endif

# Add pb2_mspm0 feature
ifeq ($(PB2_MSPM0),1)
	_RUST_ARGS_CLI += -F pb2_mspm0
endif

# Add updater feature
ifeq ($(UPDATER),1)
	_RUST_ARGS_GUI += -F updater
endif

## build: build: Build both CLI and GUI
.PHONY: build
build: build-cli build-gui

## install: install: Install both CLI and GUI. Expects things to be built first.
.PHONY: install
install: install-cli install-gui

## install: uninstall: Uninstall both CLI and GUI.
.PHONY: uninstall
uninstall: uninstall-cli uninstall-gui

## housekeeping: help: Display this help message
.PHONY: help
help:
	@python scripts/make_help.py Makefile

## housekeeping: clean: Clean the project files
.PHONY: clean
clean:
	$(info "Cleaning the project...")
	$(CARGO_PATH) clean
	rm -rf target
	rm -rf bb-imager-gui/dist
	rm -rf bb-imager-cli/dist
	rm -rf bb-imager-service/dist
	rm -rf cargo-vendor.tar.gz
	rm -rf vendor
	rm -f *.snap

## housekeeping: packaging-checks: Some checks to ensure good packaging
.PHONY: package-checks
package-checks:
	$(info Perform some checks before packaging)
ifneq (${VERSION}, ${_CARGO_TOML_VERSION})
	$(error ${VERSION} != ${_CARGO_TOML_VERSION})
endif

## housekeeping: check: Run code quality checks.
.PHONY: check
check:
	$(info "Running clippy checks")
	$(CARGO_PATH) clippy --all-targets --all-features --no-deps --workspace ${_RUST_ARGS_BASE}

## housekeeping: test: Run tests on workspace
.PHONY: test
test:
	$(info "Run workspace tests")
	$(CARGO_PATH) test --workspace --all-features ${_RUST_ARGS_BASE}

## setup: setup-debian-deps: Install debian dependencies for building. For creating packages, also run setup-packaging-deps
.PHONY: setup-debian-deps
setup-debian-deps:
	$(info "Installing Debian dependencies")
	sudo apt-get update -y
	sudo apt-get install -y --no-install-recommends libudev-dev libssl-dev libsqlite3-dev liblzma-dev

## setup: setup-fedora-deps: Install Fedora Linux dependencies for building. For creating packages, also run setup-packaging-deps
.PHONY: setup-fedora-deps
setup-fedora-deps:
	$(info "Installing Fedora dependencies")
	sudo dnf install -y openssl-devel systemd-devel xz-devel clang sqlite-devel

## setup: setup-packaging-deps: Install dependencies for generting packages.
.PHONY: setup-packaging-deps
setup-packaging-deps:
	$(info "Installing dependencies required for packaging")
	$(CARGO_PATH) install cargo-packager --locked --git https://github.com/crabnebula-dev/cargo-packager.git

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

## housekeeping: version-bump: Bump version
.PHONY: version-bump
version-bump:
	$(info Bumping version)
ifeq (${VERSION}, ${_CARGO_TOML_VERSION})
	$(error ${VERSION} == ${_CARGO_TOML_VERSION})
endif
	sed -i '/\[workspace.package\]/,/^\[/{s/^\s*version\s*=.*/version = "${VERSION}"/}' Cargo.toml
	sed -i "s/^VITE_BB_IMAGER_VERSION=.*/VITE_BB_IMAGER_VERSION=${VERSION}/" website/.env
	sed -i "s/^version: .*/version: ${VERSION}/" snapcraft.cli.yml
	sed -i '/<releases>/a \
\t\t<release version="$(VERSION)" date="$(_DATE)">\
\t\t\t<url>https://github.com/beagleboard/bb-imager-rs/releases/tag/$(VERSION)</url>\
\t\t</release>' bb-imager-gui/assets/packages/linux/flatpak/org.beagleboard.imagingutility.metainfo.xml
	cargo build
	$(info Showing Diff)
	git diff
	@while [ -z "$$CONTINUE" ]; do \
        	read -r -p "Create git commit and tag [y/N]: " CONTINUE; \
	done ; \
	[ $$CONTINUE = "y" ] || [ $$CONTINUE = "Y" ] || (echo "Aborting."; exit 1;)
	git add Cargo.toml Cargo.lock bb-imager-gui/assets/packages/linux/flatpak/org.beagleboard.imagingutility.metainfo.xml website/.env
	git commit -s -m "Bump version to ${VERSION}"
	git tag ${VERSION}

package-cli-deb: build-cli
	$(CARGO_PATH) packager -p bb-imager-cli --target $(TARGET) $(_PACKAGER_ARGS) -f deb

package-cli-pacman: build-cli
	$(CARGO_PATH) packager -p bb-imager-cli --target $(TARGET) $(_PACKAGER_ARGS) -f pacman
	rm bb-imager-cli/dist/PKGBUILD

package-gui-deb: build-gui
	$(CARGO_PATH) packager -p bb-imager-gui --target $(TARGET) ${_PACKAGER_ARGS} -f deb

package-gui-pacman: build-gui
	$(CARGO_PATH) packager -p bb-imager-gui --target $(TARGET) ${_PACKAGER_ARGS} -f pacman
	rm bb-imager-gui/dist/PKGBUILD

package-gui-appimage: build-gui
	$(CARGO_PATH) packager -p bb-imager-gui --target $(TARGET) ${_PACKAGER_ARGS} -f appimage

package-gui-dmg: build-gui
	$(CARGO_PATH) packager -p bb-imager-gui --target $(TARGET) ${_PACKAGER_ARGS} -f dmg

package-gui-wix: build-gui
	$(CARGO_PATH) packager -p bb-imager-gui --target $(TARGET) ${_PACKAGER_ARGS} -f wix

define package-linux-x86_64_aarch64
	$(info Building packages for $(1))
	$(MAKE) package-gui-pacman package-gui-deb TARGET=$(1) SYSTEM_DEPS=1
	$(MAKE) package-gui-appimage TARGET=$(1) UPDATER=1
endef

## package: package-x86_64-unknown-linux-gnu: Create all packages for x86_64-unknown-linux-gnu
.PHONY: package-x86_64-unknown-linux-gnu
package-x86_64-unknown-linux-gnu: package-checks
	$(call package-linux-x86_64_aarch64,x86_64-unknown-linux-gnu)
	$(MAKE) package-cli-deb package-cli-pacman TARGET=x86_64-unknown-linux-gnu

## package: package-aarch64-unknown-linux-gnu: Create all packages for aarch64-unknown-linux-gnu
.PHONY: package-aarch64-unknown-linux-gnu
package-aarch64-unknown-linux-gnu: package-checks
	$(call package-linux-x86_64_aarch64,aarch64-unknown-linux-gnu)
	$(MAKE) package-cli-deb package-cli-pacman TARGET=aarch64-unknown-linux-gnu PB2_MSPM0=1

## package: package-x86_64-apple-darwin: Create all packages for x86_64-apple-darwin
.PHONY: package-x86_64-apple-darwin
package-x86_64-apple-darwin: package-checks
	$(info Building packages for x86_64-apple-darwin)
	$(MAKE) package-gui-dmg TARGET=x86_64-apple-darwin UPDATER=1

## package: package-aarch64-apple-darwin: Create all packages for aarch64-apple-darwin
.PHONY: package-aarch64-apple-darwin
package-aarch64-apple-darwin: package-checks
	$(info Building packages for aarch64-apple-darwin)
	$(MAKE) package-gui-dmg TARGET=aarch64-apple-darwin UPDATER=1

## package: package-x86_64-pc-windows-msvc: Create all packages for x86_64-pc-windows-msvc
.PHONY: package-x86_64-pc-windows-msvc
package-x86_64-pc-windows-msvc: package-checks
	$(MAKE) package-gui-portable-exe package-gui-wix TARGET=x86_64-pc-windows-msvc UPDATER=1

## package: package-aarch64-pc-windows-msvc: Create all packages for aarch64-pc-windows-msvc
.PHONY: package-aarch64-pc-windows-msvc
package-aarch64-pc-windows-msvc: package-checks
	$(MAKE) package-gui-portable-exe TARGET=aarch64-pc-windows-msvc UPDATER=1

## package: package-armv7-unknown-linux-gnueabihf: Create all packages for armv7-unknown-linux-gnueabihf
.PHONY: package-armv7-unknown-linux-gnueabihf
package-armv7-unknown-linux-gnueabihf: package-checks
	$(info Building packages for armv7-unknown-linux-gnueabihf)
	$(MAKE) package-cli-deb package-cli-pacman TARGET=armv7-unknown-linux-gnueabihf

cargo-vendor.tar.gz: Cargo.lock
	$(info Create tarball of all deps)
	$(CARGO_PATH) vendor ${_RUST_ARGS_BASE}
	tar -czvf cargo-vendor.tar.gz vendor/

## housekeeping: vendor-deps: Create tarball of dependencies
.PHONY: vendor-deps
vendor-deps: cargo-vendor.tar.gz

## housekeeping: coverage: Check test coverage
.PHONY: coverage
coverage:
	$(info Check test coverage)
	$(CARGO_PATH) tarpaulin

## housekeeping: bloat: Check dependency contribution to bin size.
.PHONY: bloat
bloat:
	$(info Check dependency bloat)
	$(CARGO_PATH) bloat -p bb-imager-cli --crates --all-features --release --locked > bloat-cli.txt
	$(CARGO_PATH) bloat -p bb-imager-gui --crates --all-features --release --locked > bloat-gui.txt

.PHONY: _build-cli-bin
_build-cli-bin:
	$(info Build CLI for $(TARGET))
	$(RUST_BUILD) -p bb-imager-cli --target $(TARGET) $(_RUST_ARGS_CLI) $(_RUST_ARGS-linux)

.PHONY: _build-cli-comp
_build-cli-comp:
	$(info Generate CLI shell completion)
	mkdir -p bb-imager-cli/dist/.target/shell-comp
	$(CARGO_PATH) xtask $(_RUST_ARGS_CLI) $(_RUST_ARGS-linux) cli-shell-complete bash bb-imager-cli/dist/.target/shell-comp
	$(CARGO_PATH) xtask $(_RUST_ARGS_CLI) $(_RUST_ARGS-linux) cli-shell-complete zsh bb-imager-cli/dist/.target/shell-comp

.PHONY: _build-cli-man
_build-cli-man:
	$(info Generate CLI manpages)
	mkdir -p bb-imager-cli/dist/.target/man
	$(CARGO_PATH) xtask $(_RUST_ARGS_CLI) $(_RUST_ARGS-linux) cli-man bb-imager-cli/dist/.target/man/
	gzip -f bb-imager-cli/dist/.target/man/*

## build: build-gui: Build GUI.
.PHONY: build-gui
build-gui:
	$(info Build GUI for $(TARGET))
	$(RUST_BUILD) -p bb-imager-gui --target $(TARGET) $(_RUST_ARGS_GUI)

## build: build-cli: Build CLI and complementary stuff.
.PHONY: build-cli
build-cli: _build-cli-bin _build-cli-man _build-cli-comp

## install: install-cli: Install CLI. Intended for use in Linux
.PHONY: install-cli
install-cli:
	$(info Install CLI)
	install -Dm755 $(_CLI_BIN) $(DESTDIR)$(BINDIR)/bb-imager-cli
	mkdir -p $(DESTDIR)$(MANDIR)/man1
	install -m644 bb-imager-cli/dist/.target/man/*.gz $(DESTDIR)$(MANDIR)/man1/
	install -Dm644 $(_CLI_COMP_BASH) $(DESTDIR)$(BASH_COMPLETIONDIR)/bb-imager-cli
	install -Dm644 $(_CLI_COMP_ZSH) $(DESTDIR)$(ZSH_COMPLETIONDIR)/_bb-imager-cli

## install: uninstall-cli: Uninstall CLI. Intended for use in Linux
.PHONY: uninstall-cli
uninstall-cli:
	$(info Uninstall CLI)
	rm -f $(DESTDIR)$(BINDIR)/bb-imager-cli
	rm -f $(DESTDIR)$(MANDIR)/man1/bb-imager-cli*.gz
	rm -f $(DESTDIR)$(BASH_COMPLETIONDIR)/bb-imager-cli
	rm -f $(DESTDIR)$(ZSH_COMPLETIONDIR)/_bb-imager-cli

_install_gui:
	$(info Install GUI)
	install -Dm755 $(_GUI_BIN) $(DESTDIR)$(BINDIR)/bb-imager-gui
	install -Dm644 bb-imager-gui/assets/packages/linux/BeagleBoardImager.desktop $(DESTDIR)$(DESKTOP_DIR)/$(GUI_NAME).desktop
	desktop-file-edit --set-icon=$(GUI_NAME) $(DESTDIR)$(DESKTOP_DIR)/$(GUI_NAME).desktop
	install -Dm644 bb-imager-gui/assets/icons/icon.png $(DESTDIR)$(ICONS_DIR)/hicolor/128x128/apps/$(GUI_NAME).png
	install -Dm644 bb-imager-gui/assets/packages/linux/flatpak/org.beagleboard.imagingutility.metainfo.xml $(DESTDIR)$(METAINFO_DIR)/$(GUI_NAME).metainfo.xml

## install: install-gui: Install GUI. Intended for use in Linux.
.PHONY: install-gui
install-gui: _install_gui
	install -Dm644 bb-imager-gui/assets/packages/linux/udev/10-beagle.rules $(DESTDIR)$(UDEV_RULESDIR)/10-beagle.rules

_fetch-gui-deps:
	$(CARGO_PATH) fetch ${_RUST_ARGS_BASE} --manifest-path bb-imager-gui/Cargo.toml

## package: package-gui-flatpak: Build and install package in flatpak. Intended for use in flatpak manifest.
.PHONY: package-gui-flatpak
package-gui-flatpak:
	$(MAKE) _fetch-gui-deps build-gui _install_gui SYSTEM_DEPS=1 PREFIX=${FLATPAK_DEST} GUI_NAME=${FLATPAK_ID} OFFLINE=1

## install: uninstall-gui: Uninstall GUI. Intended for use in Linux.
.PHONY: uninstall-gui
uninstall-gui:
	$(info Uninstall GUI)
	rm -f $(DESTDIR)$(BINDIR)/bb-imager-gui
	rm -f $(DESTDIR)$(UDEV_RULESDIR)/10-beagle.rules
	rm -f $(DESTDIR)$(DESKTOP_DIR)/$(GUI_NAME).desktop
	rm -f $(DESTDIR)$(ICONS_DIR)/hicolor/128x128/apps/$(GUI_NAME).png
	rm -f $(DESTDIR)$(METAINFO_DIR)/$(GUI_NAME).metainfo.xml

## package: package-gui-portable-exe: Build portable exe for GUI.
.PHONY: package-gui-portable-exe
package-gui-portable-exe: build-gui
	$(info Building portable windows exe for $(TARGET))
	mkdir -p bb-imager-gui/dist
	cp $(_GUI_BIN) $(_GUI_PORTABLE_EXE)

## package: package-host: Build all packages for host platform.
.PHONY: package-host
package-host: package-$(_HOST_TARGET)

## package: package-cli-snap: Build snap package for CLI.
.PHONY: package-cli-snap
package-cli-snap:
	$(info Build snap package for CLI)
	ln -sf snapcraft.cli.yaml snapcraft.yaml
	snapcraft pack -v
	unlink snapcraft.yaml

## package: package-gui-snap: Build snap package for GUI.
.PHONY: package-gui-snap
package-gui-snap:
	$(info Build snap package for GUI)
	ln -sf snapcraft.gui.yaml snapcraft.yaml
	snapcraft pack -v
	unlink snapcraft.yaml
