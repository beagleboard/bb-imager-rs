# End-to-End Testing Guide

This guide covers the end-to-end (E2E) testing framework for BeagleBoard Imager, which tests all flashing workflows across multiple platforms.

## Overview

The E2E test suite validates:
- **SD Card Flashing**: Writing images to SD cards on Linux, Windows, and macOS
- **BCF Flashing**: Programming BeagleConnect Freedom (CC1352P7 and MSP430)
- **DFU Flashing**: Device Firmware Update for USB devices

## Test Structure

```
e2e-tests/
├── Cargo.toml              # E2E tests package
└── tests/
    ├── e2e.rs              # Main test module
    └── e2e/
        ├── common.rs       # Common utilities for all tests
        ├── sd_flash.rs     # SD card flashing tests
        ├── bcf_flash.rs    # BCF flashing tests
        └── dfu_flash.rs    # DFU flashing tests
```

## Running Tests

### All E2E Tests

Run the complete E2E test suite:

```bash
make test-e2e
# Or directly:
cargo test -p e2e-tests --features all
```

### Platform-Specific Tests

#### SD Card Tests

```bash
# Run all SD card tests
make test-e2e-sd
# Or: cargo test -p e2e-tests sd_flash --features sd

# Run specific SD test
cargo test -p e2e-tests test_sd_flash_uncompressed --features sd
```

#### BCF Tests

```bash
# Run BCF CC1352P7 tests
cargo test -p e2e-tests bcf_flash --features bcf

# Run BCF MSP430 tests
cargo test -p e2e-tests bcf_flash --features bcf_msp430

# Run all BCF tests
make test-e2e-bcf
# Or: cargo test -p e2e-tests bcf_flash --features bcf,bcf_msp430
```

#### DFU Tests

```bash
# Run all DFU tests
make test-e2e-dfu
# Or: cargo test -p e2e-tests dfu_flash --features dfu
```

### Test Options

#### Run with verbose output

```bash
cargo test -p e2e-tests -- --nocapture
```

#### Run tests in serial (recommended for device tests)

```bash
cargo test -p e2e-tests -- --test-threads=1
```

#### Skip tests that require physical devices

Most tests that require physical devices (BCF, DFU) will automatically skip if no device is detected.

## Test Coverage

### SD Card Tests

| Test | Description | Platforms |
|------|-------------|-----------|
| `test_sd_flash_uncompressed` | Flash uncompressed image to virtual SD | All |
| `test_sd_flash_compressed` | Flash compressed (xz) image to virtual SD | All |
| `test_sd_flash_with_customization` | Flash with sysconf customization | All |
| `test_sd_flash_cancellation` | Test cancellation mechanism | All |
| `test_sd_format` | Format SD card | All |

### BCF Tests

| Test | Description | Requirements |
|------|-------------|--------------|
| `test_bcf_flash_with_verify` | Flash CC1352P7 with verification | BCF device |
| `test_bcf_flash_no_verify` | Flash CC1352P7 without verification | BCF device |
| `test_bcf_list_destinations` | List BCF devices | None |
| `test_msp430_flash` | Flash MSP430 firmware | BCF device |
| `test_msp430_list_destinations` | List MSP430 targets | None |

### DFU Tests

| Test | Description | Requirements |
|------|-------------|--------------|
| `test_dfu_list_destinations` | List DFU devices | None |
| `test_dfu_flash_single_firmware` | Flash single firmware | DFU device |
| `test_dfu_flash_multiple_firmwares` | Flash multiple firmwares | DFU device |
| `test_dfu_flash_with_identifier` | Flash using device identifier | DFU device |
| `test_dfu_flash_cancellation` | Test cancellation | DFU device |

## CI/CD Integration

The E2E tests run automatically in GitHub Actions:

### Workflow: `.github/workflows/e2e.yml`

The workflow runs on:
- Pull requests to `main`
- Pushes to `main`
- Manual trigger via `workflow_dispatch`

#### Jobs

1. **e2e-sd**: SD card tests on all platforms
2. **e2e-bcf**: BCF tests on all platforms (allowed to fail if no device)
3. **e2e-dfu**: DFU tests on all platforms (allowed to fail if no device)
4. **e2e-all**: All tests with all features enabled

#### Platform Matrix

- Ubuntu Latest
- Windows Latest
- macOS Latest

## Writing New Tests

### Test Template

```rust
#[tokio::test]
async fn test_new_feature() {
    // 1. Setup: Create test fixtures
    let img_path = common::create_test_image(IMAGE_SIZE)
        .expect("Failed to create test image");
    
    // 2. Execute: Run the operation being tested
    let result = perform_operation().await;
    
    // 3. Cleanup: Remove test files
    common::cleanup_test_file(&img_path).ok();
    
    // 4. Assert: Verify results
    assert!(result.is_ok(), "Operation failed: {:?}", result.err());
}
```

### Helper Functions

The `common` module provides utilities:

- `create_test_image(size)`: Create a test image file
- `create_compressed_test_image(size)`: Create compressed (xz) image
- `create_virtual_sd_card(size)`: Create virtual SD card for testing
- `verify_written_image(written, original)`: Verify image was written correctly
- `cleanup_test_file(path)`: Clean up test files

## Platform-Specific Considerations

### Linux

- Virtual device tests run without privileges
- Real device tests may require `sudo` or udev rules
- Install dependencies: `make setup-debian-deps`

### Windows

- Ensure proper USB drivers installed for BCF/DFU devices
- Some tests may require administrator privileges
- Visual Studio Build Tools may be required

### macOS

- Security permissions may be needed for device access
- Use `authopen` for privileged SD card access (GUI only)
- Install dependencies: `brew install ...` (see CONTRIBUTING.adoc)

## Troubleshooting

### Tests Fail Due to Missing Devices

Tests requiring physical devices (BCF, DFU) will skip automatically. This is expected behavior in CI.

### Permission Denied Errors

```bash
# Linux: Add user to required groups
sudo usermod -aG dialout,disk $USER

# Or run with sudo (not recommended for CI)
sudo cargo test -p e2e-tests
```

### Timeout Issues

Increase timeout for slow operations:

```bash
# Set test timeout to 10 minutes
cargo test -p e2e-tests -- --test-timeout=600
```

### Clean Build

If tests behave unexpectedly:

```bash
cargo clean
cargo test -p e2e-tests
```

## Continuous Improvement

### Adding New Test Cases

1. Identify the feature to test
2. Create test in appropriate module (`sd_flash.rs`, `bcf_flash.rs`, `dfu_flash.rs`)
3. Use common utilities where possible
4. Document test purpose and requirements
5. Ensure cleanup is performed

### Improving Test Coverage

Run with coverage tool:

```bash
# Using cargo-tarpaulin
cargo install cargo-tarpaulin
cargo tarpaulin -p e2e-tests --features all --out Html
```

### Performance Testing

For performance-critical operations:

```rust
#[tokio::test]
async fn test_flash_performance() {
    let start = std::time::Instant::now();
    // ... perform operation
    let duration = start.elapsed();
    assert!(duration.as_secs() < 60, "Operation took too long: {:?}", duration);
}
```

## Contributing

When adding new flashing features:

1. Add corresponding E2E tests
2. Update this documentation
3. Ensure tests pass on all platforms
4. Add CI workflow if needed

See [CONTRIBUTING.adoc](../CONTRIBUTING.adoc) for more details.

