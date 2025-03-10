_CARGO_DEB_FLAGS := ' --no-build --no-strip'

# Create Linux debian package
_package-linux-deb PKG TARGET OUTPUT: _deb_prechecks
	@echo "Create {{ PKG }} Linux debian package for {{ TARGET }}"
	{{ if file_name(RUST_BUILDER) == 'cross' { \
		RUST_BUILDER + ' run --target ' + TARGET + ' -- "cargo deb --target ' + TARGET + ' -p ' + PKG + ' -o ' + OUTPUT + _CARGO_DEB_FLAGS + '"' \
	} else { \
		RUST_BUILDER + ' deb --target ' + TARGET + ' -p ' + PKG + ' -o ' + OUTPUT + _CARGO_DEB_FLAGS \
	} }}
