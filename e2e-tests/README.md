# E2E Tests for BeagleBoard Imager

This package contains end-to-end tests for all BeagleBoard Imager flashing workflows.

## Features

- `sd`: SD card flashing tests
- `bcf`: BeagleConnect Freedom CC1352P7 tests
- `bcf_msp430`: BeagleConnect Freedom MSP430 tests
- `dfu`: DFU (Device Firmware Update) tests
- `all`: Enable all test features

## Running Tests

```bash
# Run all tests
cargo test -p e2e-tests --features all

# Run specific platform tests
cargo test -p e2e-tests --features sd
cargo test -p e2e-tests --features bcf,bcf_msp430
cargo test -p e2e-tests --features dfu

# Run with make
make test-e2e
make test-e2e-sd
make test-e2e-bcf
make test-e2e-dfu
```

## Test Structure

- `tests/e2e.rs` - Main test module
- `tests/e2e/common.rs` - Common test utilities
- `tests/e2e/sd_flash.rs` - SD card flashing tests
- `tests/e2e/bcf_flash.rs` - BCF flashing tests
- `tests/e2e/dfu_flash.rs` - DFU flashing tests

## Documentation

See [../docs/E2E_TESTING.md](../docs/E2E_TESTING.md) for complete testing documentation.

