# Map Rust targets with Appimage Arch
APPIMAGE_ARCH_x86_64-unknown-linux-gnu = x86_64
APPIMAGE_ARCH_aarch64-unknown-linux-gnu = aarch64
APPIMAGE_ARCH_armv7-unknown-linux-gnueabihf = armhf

RUST_GUI_FEATURE_ARGS = $(RUST_FEATURE_ARGS)

ifdef PB2_MSPM0
	RUST_GUI_FEATURE_ARGS += -F pb2_mspm0
endif

build-gui-%:
	$(info "Building GUI for $*")
	$(RUST_BUILDER) build --release --target $* -p bb-imager-gui $(RUST_GUI_FEATURE_ARGS)

# Special rule for macos universal binary
build-gui-universal-apple-darwin: build-gui-aarch64-apple-darwin build-gui-x86_64-apple-darwin
	$(info "Building GUI for universal-apple-darwin")
	mkdir -p target/universal-apple-darwin/release
	lipo -create target/x86_64-apple-darwin/release/bb-imager-gui target/aarch64-apple-darwin/release/bb-imager-gui -output target/universal-apple-darwin/release/bb-imager-gui

package-gui-linux-appimage-%: build-gui-%
	$(info "Create GUI Linux Appimage $*")
	mkdir -p ${RELEASE_DIR_LINUX}/$*/AppDir/usr/bin
	cp ${GUI_ASSETS_LINUX}/appimage/AppRun ${RELEASE_DIR_LINUX}/$*/AppDir/AppRun
	cp target/$*/release/bb-imager-gui ${RELEASE_DIR_LINUX}/$*/AppDir/usr/bin/
	cp ${GUI_ASSETS_LINUX}/BeagleBoardImager.desktop ${RELEASE_DIR_LINUX}/$*/AppDir/
	sed -i "s/^X-AppImage-Version=0.0.0/X-AppImage-Version=${VERSION}/" ${RELEASE_DIR_LINUX}/$*/AppDir/BeagleBoardImager.desktop
	cp ${GUI_ASSETS}/icons/icon.png ${RELEASE_DIR_LINUX}/$*/AppDir/bb-imager.png
	ARCH=${APPIMAGE_ARCH_$*} $(APPIMAGETOOL) --appimage-extract-and-run ${RELEASE_DIR_LINUX}/$*/AppDir ${RELEASE_DIR_LINUX}/$*/BeagleBoardImager.AppImage
	rm -rf ${RELEASE_DIR_LINUX}/$*/AppDir

package-gui-linux-deb-%: build-gui-%
	$(info "Create GUI Linux deb $*")
ifeq ($(RUST_BUILDER_NAME), cross)
	$(CROSS_UTIL) run --target $* -- "cargo deb --target $* -p bb-imager-gui --no-build -o ${RELEASE_DIR_LINUX}/$*/bb-imager-gui.deb --no-strip"
else
	$(RUST_BUILDER) deb --target $* -p bb-imager-gui --no-build -o ${RELEASE_DIR_LINUX}/$*/bb-imager-gui.deb --no-strip
endif

package-gui-darwin-dmg-%: build-gui-%
	$(info "Create GUI Darwin dmg $*")
	sed -i'.bak' -e "s/0\.0\.0/${CI_COMMIT_TAG}/g" -e "s/fffffff/${CI_COMMIT_SHA:0:7}/g" ${GUI_ASSETS_DARWIN}/Info.plist
	mkdir -p ${RELEASE_DIR_DARWIN}/$*/AppDir/BeagleBoardImager.app/Contents/{MacOS,Resources}
	cp ${GUI_ASSETS_DARWIN}/Info.plist ${RELEASE_DIR_DARWIN}/$*/AppDir/BeagleBoardImager.app/Contents/
	cp ${GUI_ASSETS}/icons/icon.icns ${RELEASE_DIR_DARWIN}/$*/AppDir/BeagleBoardImager.app/Contents/Resources/
	cp target/$*/release/bb-imager-gui ${RELEASE_DIR_DARWIN}/$*/AppDir/BeagleBoardImager.app/Contents/MacOS/
	create-dmg \
		--volname "BeagleBoardImager Installer" \
		--volicon "${GUI_ASSETS}/icons/icon.icns" \
		--window-pos 200 120 \
		--window-size 800 400 \
		--icon-size 100 \
		--icon "BeagleBoardImager.app" 200 190 \
		--hide-extension "BeagleBoardImager.app" \
		--app-drop-link 600 185 \
		--hdiutil-verbose \
		--skip-jenkins \
		"${RELEASE_DIR_DARWIN}/$*/BeagleBoardImager.dmg" \
		"${RELEASE_DIR_DARWIN}/$*/AppDir/"
	rm -rf ${RELEASE_DIR_DARWIN}/$*/AppDir/

package-gui-windows-zip-%: build-gui-%
	$(info "Create GUI Windows zip $*")
	mkdir -p ${RELEASE_DIR_WINDOWS}/$*
	zip -j ${RELEASE_DIR_WINDOWS}/$*/bb-imager-gui.zip target/$*/release/bb-imager-gui.exe
