_appimage_prechecks TARGET:
    @echo "Checking appimagetool"
    {{ if shell('command -v $1', APPIMAGETOOL) == '' { error('Appimagetool is missing. Try running `just setup-appimage`') } else { '' } }}


_deb_prechecks:
    @echo "Checking if cargo-deb is installed"
    {{ if which('cargo-deb') == '' { error('cargo-deb is required to build Debian package. Try running `just setup-deb`') } else { '' } }}


# Run code checks
[group('housekeeping')]
check:
    @echo "Run clippy checks"
    {{ RUST_BUILDER }} clippy --all-targets --all-features --no-deps --workspace

