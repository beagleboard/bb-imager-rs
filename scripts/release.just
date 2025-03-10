# Generate all supported release for the Linux target
_release-linux TARGET=_HOST_TARGET: (package-cli-linux-xz TARGET) (package-gui-linux-appimage TARGET) (package-gui-linux-deb TARGET)
	{{ if PB2_MSPM0 == '1' { 'just package-service-linux-xz ' + TARGET + ' package-service-linux-deb ' + TARGET } else { '' } }}

# Generate all supported release for the Windows target
_release-windows TARGET=_HOST_TARGET: (package-cli-windows-zip TARGET) (package-gui-windows-zip TARGET)

# Generate all supported release for the macOS target
_release-darwin TARGET=_HOST_TARGET: (package-cli-darwin-zip TARGET) (package-gui-darwin-dmg TARGET)

# Generate all supported packages for the target
[group('packaging')]
release TARGET=_HOST_TARGET:
	@echo "Generating all release for {{ TARGET }}"
	{{ if TARGET =~ "windows" { \
		'just _release-windows ' + TARGET \
	} else if TARGET =~ "darwin" { \
		'just _release-darwin ' + TARGET \
	} else if TARGET =~ "linux" { \
		'just _release-linux ' + TARGET \
	} else { error('Unsupported Platform') } }}
