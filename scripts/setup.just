# Setup for building appimages
[group('setup')]
setup-appimage INSTALL_DIR=_EXE_DIR:
	@echo "Install Appimagetool at {{ INSTALL_DIR }}"
	wget https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$(uname -m).AppImage -O {{INSTALL_DIR}}/appimagetool
	chmod +x {{INSTALL_DIR}}/appimagetool


# Setup for building debian packages
[group('setup')]
setup-deb:
	@echo "Install cargo-deb"
	{{ if which('cargo-binstall') == '' { 'cargo install --locked -y cargo-deb' } else { 'cargo binstall -y cargo-deb ' } }}
