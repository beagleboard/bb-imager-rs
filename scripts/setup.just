_check-appimagetool:
	{{ if which('appimagetool') == '' { '' } else { error('Appimagetool is already available') } }}


# Setup for building appimages
[group('setup')]
setup-appimage INSTALL_DIR=_EXE_DIR: _check-appimagetool
	@echo "Install Appimagetool at {{ INSTALL_DIR }}"
	wget https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$(uname -m).AppImage -O {{INSTALL_DIR}}/appimagetool
	chmod +x {{INSTALL_DIR}}/appimagetool


# Setup for building debian packages
[group('setup')]
setup-deb:
	@echo "Install cargo-deb"
	{{ if which('cargo-binstall') == '' { 'cargo install --locked -y cargo-deb' } else { 'cargo binstall -y cargo-deb ' } }}


# Setup for building dmg
[group('setup')]
setup-dmg:
	@echo "Install create-dmg"
	brew install create-dmg


# Check if Rust is already installed
_check-rust:
	{{ if which('cargo') == '' { '' } else { error('Rust is already available') } }}


# Setup rust toolcahin
[group('setup')]
setup-rust: _check-rust
	@echo "Setting up rust using rustup"
	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
