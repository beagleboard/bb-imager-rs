RUST_BUILDER ?= cross
APPIMAGETOOL ?= appimagetool

# Build Appimage for BeagleBoardImager GUI
define appimage
	mkdir -p release/linux/$(1)/AppDir/usr/bin
	cp assets/AppRun release/linux/$(1)/AppDir/AppRun
	cp target/$(1)/release/bb-imager-gui release/linux/$(1)/AppDir/usr/bin/
	cp gui/BeagleBoardImager.desktop release/linux/$(1)/AppDir/
	cp gui/icon.png release/linux/$(1)/AppDir/
	ARCH=$(2) $(APPIMAGETOOL) --appimage-extract-and-run release/linux/$(1)/AppDir release/linux/$(1)/BeagleBoardImager.AppImage
	rm -rf release/linux/$(1)/AppDir
endef

# Build Executable for BeagleBoardImager CLI
define cli
	mkdir -p release/linux/$(1)
	xz -vc target/$(1)/release/bb-imager-cli > release/linux/$(1)/bb-imager-cli.xz
endef

clean:
	rm -rf release

build-windows-x86_64:
	$(info "Building Windows release for x86_64")
	$(RUST_BUILDER) build --release --target x86_64-pc-windows-gnu

build-linux-x86_64:
	$(info "Building Linux release for x86_64")
	$(RUST_BUILDER) build --release --target x86_64-unknown-linux-gnu

build-linux-aarch64:
	$(info "Building Linux release for aarch64")
	$(RUST_BUILDER) build --release --target aarch64-unknown-linux-gnu

build-linux-arm:
	$(info "Building Linux release for arm")
	$(RUST_BUILDER) build --release --target armv7-unknown-linux-gnueabihf

build-darwin-x86_64:
	$(info "Building MacOS release for x86_64")
	$(RUST_BUILDER) build --release --target x86_64-apple-darwin

build-darwin-aarch64:
	$(info "Building MacOS release for aarch64")
	$(RUST_BUILDER) build --release --target aarch64-apple-darwin

release-windows-x86_64: build-windows-x86_64
	$(info "Generating Windows release for x86_64")
	mkdir -p release/windows/x86_64-pc-windows-gnu
	zip -j release/windows/x86_64-pc-windows-gnu/bb-imager-cli.zip target/x86_64-pc-windows-gnu/release/bb-imager-cli.exe
	zip -j release/windows/x86_64-pc-windows-gnu/bb-imager-gui.zip target/x86_64-pc-windows-gnu/release/bb-imager-gui.exe

release-linux-gui-appimage-x86_64: build-linux-x86_64
	$(info "Generating Linux Appimage release for x86_64")
	$(call appimage,x86_64-unknown-linux-gnu,x86_64)

release-linux-gui-appimage-aarch64: build-linux-aarch64
	$(info "Generating Linux Appimage release for aarch64")
	$(call appimage,aarch64-unknown-linux-gnu,aarch64)

release-linux-gui-appimage-arm: build-linux-arm
	$(info "Generating Linux Appimage release for aarch64")
	$(call appimage,armv7-unknown-linux-gnueabihf,armhf)

release-linux-cli-x86_64: build-linux-x86_64
	$(info "Generating Linux CLI release for x86_64")
	$(call cli,x86_64-unknown-linux-gnu)

release-linux-cli-aarch64: build-linux-aarch64
	$(info "Generating Linux CLI release for x86_64")
	$(call cli,aarch64-unknown-linux-gnu)

release-linux-cli-arm: build-linux-arm
	$(info "Generating Linux CLI release for x86_64")
	$(call cli,armv7-unknown-linux-gnueabihf)

release-darwin-x86_64: build-darwin-x86_64
	$(info "Generating MacOS release for x86_64")
	mkdir -p release/darwin/x86_64-apple-darwin
	zip -j release/darwin/x86_64-apple-darwin/bb-imager-cli.zip target/x86_64-apple-darwin/release/bb-imager-cli

release-darwin-aarch64: build-darwin-aarch64
	$(info "Generating MacOS release for aarch64")
	mkdir -p release/darwin/aarch64-apple-darwin
	zip -j release/darwin/aarch64-apple-darwin/bb-imager-cli.zip target/aarch64-apple-darwin/release/bb-imager-cli

release-linux-x86_64: release-linux-cli-x86_64 release-linux-gui-appimage-x86_64

release-linux-aarch64: release-linux-cli-aarch64 release-linux-gui-appimage-aarch64

release-linux-arm: release-linux-cli-arm release-linux-gui-appimage-arm

release-linux: release-linux-x86_64 release-linux-aarch64 release-linux-arm

# TODO: Add aarch64 windows.
release-windows: release-windows-x86_64

release-darwin: release-darwin-x86_64 release-darwin-aarch64
