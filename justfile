#!/usr/bin/env just --justfile

set unstable := true

import 'scripts/checks.just'
import 'scripts/setup.just'
import 'scripts/packaging.just'
import 'scripts/release.just'
import 'bb-imager-gui/justfile'
import 'bb-imager-cli/justfile'
import 'bb-imager-service/justfile'

[private]
_CARGO_PATH := which("cargo")
[private]
_COMMIT_HASH := shell('git rev-parse HEAD')
[private]
_HOST_TARGET := arch() + if os() == 'linux' { '-unknown-linux-gnu' } else if os() == 'macos' { '-apple-darwin' } else if os() == 'windows' { '-pc-windows-gnu' } else { error("Unsupported Platform") }
[private]
_RELEASE_DIR := source_directory() / 'release'
[private]
_EXE_DIR := if os() == 'linux' { executable_directory() } else { '' }

# Public arguments

RUST_BUILDER := env("RUST_BUILDER", _CARGO_PATH)
PB2_MSPM0 := env("PB2_MSPM0", '0')
BCF_CC1352 := env("BCF_CC1352", '0')
BCF_MSP430 := env("BCF_MSP430", '0')
APPIMAGETOOL := env("APPIMAGETOOL", which('appimagetool'))
VERSION := env('VERSION', shell('grep "version =" Cargo.toml | sed "s/version = \"\(.*\)\"/\1/"'))

# default recipe to display help information
default:
    @just --list

# Run tests on workspace
[group('housekeeping')]
test:
    @echo "Run workspace tests"
    {{ RUST_BUILDER }} test --workspace

# Clean Artifacts
[group('housekeeping')]
clean:
    {{ _CARGO_PATH }} clean
    rm -rf release
