# Run 'make help' to see guidance on usage of this Makefile

_TARGET_ARCH = $(shell echo ${TARGET} | cut -d'-' -f1)
_CARGO_TOML_VERSION = $(shell grep 'version =' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
_DATE = $(shell date +%F)
# Rust args common for GUI and CLI across all targets and packages
_RUST_ARGS_BASE = --locked --verbose
_RUST_ARGS = ${_RUST_ARGS_BASE} -r -F bcf_cc1352p7,bcf_msp430,zepto_uart
_RUST_ARGS-linux = -F zepto_i2c
_RUST_ARGS_CLI = ${_RUST_ARGS} -F dfu
_RUST_ARGS_CLI-aarch64-unknown-linux-gnu = -F pb2_mspm0
_PACKAGER_ARGS = -r -vvv --verbose

## variable: CARGO_PATH: Path to cargo binary
CARGO_PATH ?= $(shell which cargo)
## variable: RUST_BUILDER: The Rust builder to use. Possble choices - cargo (default), cross.
RUST_BUILDER ?= $(CARGO_PATH)
## variable: VERSION: Release versions for bb-imager-cli and bb-imager-gui
VERSION ?= $(_CARGO_TOML_VERSION)
## variable: NO_BUILD: Do not build any packages. Useful for cross builds in CI.
NO_BUILD ?= 0

# Allow skipping build step
ifeq ($(NO_BUILD),1)
RUST_BUILD = @true
else
RUST_BUILD = $(RUST_BUILDER) build
endif

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

# Will need updates if CLI starts being built outside linux
define build-cli
	$(info Generate CLI Manpages)
	rm -rf bb-imager-cli/dist/.target/man
	mkdir -p bb-imager-cli/dist/.target/man
	$(CARGO_PATH) xtask ${_RUST_ARGS_CLI} ${_RUST_ARGS-linux} $(_RUST_ARGS_CLI-$(1)) cli-man bb-imager-cli/dist/.target/man/
	gzip bb-imager-cli/dist/.target/man/*

	$(info Generate CLI completion)
	rm -rf bb-imager-cli/dist/.target/shell-comp
	mkdir -p bb-imager-cli/dist/.target/shell-comp
	$(CARGO_PATH) xtask ${_RUST_ARGS_CLI} ${_RUST_ARGS-linux} $(_RUST_ARGS_CLI-$(1)) cli-shell-complete zsh bb-imager-cli/dist/.target/shell-comp
	$(CARGO_PATH) xtask ${_RUST_ARGS_CLI} ${_RUST_ARGS-linux} $(_RUST_ARGS_CLI-$(1)) cli-shell-complete bash bb-imager-cli/dist/.target/shell-comp

	$(info Build CLI for $(1))
	$(RUST_BUILD) -p bb-imager-cli --target $(1) ${_RUST_ARGS_CLI} ${_RUST_ARGS-linux} $(_RUST_ARGS_CLI-$(1))
	$(CARGO_PATH) packager -p bb-imager-cli --target $(1) ${_PACKAGER_ARGS} -f deb,pacman
	rm bb-imager-cli/dist/PKGBUILD
endef

define package-linux-x86_64_aarch64
	$(info Building packages for $(1))
	$(call build-cli,$(1))
	$(RUST_BUILD) -p bb-imager-gui --target $(1) ${_RUST_ARGS} ${_RUST_ARGS-linux} --no-default-features -F system-deps
	$(CARGO_PATH) packager -p bb-imager-gui --target $(1) ${_PACKAGER_ARGS} -f deb,pacman
	$(RUST_BUILD) -p bb-imager-gui --target $(1) ${_RUST_ARGS} ${_RUST_ARGS-linux} -F updater
	$(CARGO_PATH) packager -p bb-imager-gui --target $(1) ${_PACKAGER_ARGS} -f appimage
	rm bb-imager-gui/dist/PKGBUILD
endef

define package-apple-x86_64_aarch64
	$(info Building packages for $(1))
	$(RUST_BUILD) -p bb-imager-gui --target $(1) ${_RUST_ARGS} -F updater
	$(CARGO_PATH) packager -p bb-imager-gui --target $(1) ${_PACKAGER_ARGS} -f dmg
endef

define package-windows-x86_64_aarch64
	$(info Building packages for $(1))
	$(RUST_BUILD) -p bb-imager-gui --target $(1) ${_RUST_ARGS} -F updater
	mkdir -p bb-imager-gui/dist
	cp target/$(1)/release/bb-imager-gui.exe bb-imager-gui/dist/bb-imager-gui_${VERSION}_$(word 1,$(subst -, ,$(1))).exe
endef

## package: package-x86_64-unknown-linux-gnu: Create all packages for x86_64-unknown-linux-gnu
.PHONY: package-x86_64-unknown-linux-gnu
package-x86_64-unknown-linux-gnu: package-checks
	$(call package-linux-x86_64_aarch64,x86_64-unknown-linux-gnu)

## package: package-aarch64-unknown-linux-gnu: Create all packages for aarch64-unknown-linux-gnu
.PHONY: package-aarch64-unknown-linux-gnu
package-aarch64-unknown-linux-gnu: package-checks
	$(call package-linux-x86_64_aarch64,aarch64-unknown-linux-gnu)

## package: package-x86_64-apple-darwin: Create all packages for x86_64-apple-darwin
.PHONY: package-x86_64-apple-darwin
package-x86_64-apple-darwin: package-checks
	$(call package-apple-x86_64_aarch64,x86_64-apple-darwin)

## package: package-aarch64-apple-darwin: Create all packages for aarch64-apple-darwin
.PHONY: package-aarch64-apple-darwin
package-aarch64-apple-darwin: package-checks
	$(call package-apple-x86_64_aarch64,aarch64-apple-darwin)

## package: package-x86_64-pc-windows-msvc: Create all packages for x86_64-pc-windows-msvc
.PHONY: package-x86_64-pc-windows-msvc
package-x86_64-pc-windows-msvc: package-checks
	$(call package-windows-x86_64_aarch64,x86_64-pc-windows-msvc)
	$(CARGO_PATH) packager -p bb-imager-gui --target x86_64-pc-windows-msvc ${_PACKAGER_ARGS} -f wix

## package: package-aarch64-pc-windows-msvc: Create all packages for aarch64-pc-windows-msvc
.PHONY: package-aarch64-pc-windows-msvc
package-aarch64-pc-windows-msvc: package-checks
	$(call package-windows-x86_64_aarch64,aarch64-pc-windows-msvc)

## package: package-armv7-unknown-linux-gnueabihf: Create all packages for armv7-unknown-linux-gnueabihf
.PHONY: package-armv7-unknown-linux-gnueabihf
package-armv7-unknown-linux-gnueabihf: package-checks
	$(info Building packages for armv7-unknown-linux-gnueabihf)
	$(call build-cli,armv7-unknown-linux-gnueabihf)

## package: package-flatpak: Build and install package in flatpak. Intended for use in flatpak manifest.
.PHONY: build-flatpak
package-flatpak:
	$(info Building for flatpak)
	cargo --offline fetch --verbose --manifest-path bb-imager-gui/Cargo.toml
	cargo --offline build -p bb-imager-gui ${_RUST_ARGS} ${_RUST_ARGS-linux} --no-default-features -F system-deps
	install -Dm755 ./target/release/bb-imager-gui -t /app/bin/
	install -Dm644 ./bb-imager-gui/assets/packages/linux/BeagleBoardImager.desktop /app/share/applications/${FLATPAK_ID}.desktop
	desktop-file-edit --set-icon=${FLATPAK_ID} /app/share/applications/${FLATPAK_ID}.desktop
	install -Dm644 ./bb-imager-gui/assets/icons/icon.png /app/share/icons/hicolor/128x128/apps/${FLATPAK_ID}.png
	install -Dm644 ./bb-imager-gui/assets/packages/linux/flatpak/org.beagleboard.imagingutility.metainfo.xml ${FLATPAK_DEST}/share/metainfo/${FLATPAK_ID}.metainfo.xml

cargo-vendor.tar.gz: Cargo.lock
	$(info Create tarball of all deps)
	$(CARGO_PATH) vendor
	tar -czvf cargo-vendor.tar.gz vendor/

## housekeeping: vendor-deps: Create tarball of dependencies
.PHONY: vendor-deps
vendor-deps: cargo-vendor.tar.gz

## housekeeping: coverage: Check test coverage
.PHONY: coverage
coverage:
	$(info Check test coverage)
	cargo tarpaulin

## housekeeping: bloat: Check dependency contribution to bin size.
.PHONY: bloat
bloat:
	$(info Check dependency bloat)
	cargo bloat -p bb-imager-cli --crates --all-features --release --locked > bloat-cli.txt
	cargo bloat -p bb-imager-gui --crates --all-features --release --locked > bloat-gui.txt
