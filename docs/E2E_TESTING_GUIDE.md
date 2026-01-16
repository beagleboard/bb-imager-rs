# E2E Testing Guide for BeagleBoard Imager

This guide provides comprehensive information about end-to-end (E2E) testing for BeagleBoard Imager across all supported platforms.

## Table of Contents

1. [Overview](#overview)
2. [Platform-Specific Testing](#platform-specific-testing)
3. [Test Categories](#test-categories)
4. [Running Tests](#running-tests)
5. [CI/CD Integration](#cicd-integration)
6. [Hardware Requirements](#hardware-requirements)
7. [Troubleshooting](#troubleshooting)
8. [Contributing](#contributing)

## Overview

The E2E test suite validates complete flashing workflows for:

- **SD Card Flashing**: OS image writing with customization support
- **BeagleConnect Freedom (BCF)**: CC1352P7 and MSP430 firmware flashing
- **DFU Devices**: USB Device Firmware Update protocol

### Platform Coverage Matrix

| Feature | Linux | Windows | macOS | Notes |
|---------|-------|---------|-------|-------|
| SD Card Flashing | ✅ | ✅ | ✅ | Full platform support |
| BCF CC1352P7 | ✅ | ✅ | ✅ | libusb/WinUSB required |
| BCF MSP430 | ✅ | ✅ | ✅ | HID access required |
| DFU Flashing | ✅ | ✅ | ✅ | USB DFU support |
| Virtual Devices | ✅ | ✅ | ✅ | No hardware needed |
| Physical Devices | ✅ | ✅ | ✅ | Hardware required |

## Platform-Specific Testing

### Linux Testing

#### System Requirements

```bash
# Install required system libraries
sudo apt-get update
sudo apt-get install -y \
    libudev-dev \
    libusb-1.0-0-dev \
    pkg-config
```

#### Permission Setup

For physical device testing:

```bash
# Add user to plugdev group
sudo usermod -a -G plugdev $USER

# Install udev rules (if available)
sudo cp bb-imager-service/assets/*.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger

# Log out and back in for group changes to take effect
```

#### Running Tests

```bash
# Virtual device tests (no special permissions needed)
cargo test -p e2e-tests --features sd

# Physical device tests (may need sudo)
sudo -E cargo test -p e2e-tests --features sd

# BCF/DFU tests
cargo test -p e2e-tests --features bcf,bcf_msp430,dfu
```

#### Platform-Specific Test Examples

```rust
#[cfg(target_os = "linux")]
#[tokio::test]
async fn test_sd_flash_linux_virtual_device() {
    // Test Linux-specific SD card handling
    // Uses virtual device (/tmp/virtual_sd_*.img)
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn test_sd_format_linux_ext4() {
    // Test Linux ext4 formatting
}
```

### Windows Testing

#### System Requirements

1. **Visual Studio Build Tools** or Visual Studio with C++ support
2. **WinUSB Drivers** for BCF and DFU devices
   - Use [Zadig](https://zadig.akeo.ie/) to install WinUSB driver

#### Driver Installation

For BCF/DFU devices:

1. Connect the device
2. Open Zadig
3. Select the device from the dropdown
4. Choose "WinUSB" as the target driver
5. Click "Install Driver" or "Replace Driver"

#### Running Tests

```powershell
# In PowerShell or Command Prompt
cd e2e-tests

# Run all tests
cargo test --test e2e --all-features

# Run specific tests
cargo test --test e2e --features sd
cargo test --test e2e --features bcf,bcf_msp430
cargo test --test e2e --features dfu
```

#### Administrator Privileges

Some tests may require administrator privileges:

```powershell
# Run PowerShell as Administrator, then:
cd path\to\bb-imager-rs\e2e-tests
cargo test --test e2e --features sd -- --nocapture
```

#### Platform-Specific Test Examples

```rust
#[cfg(target_os = "windows")]
#[tokio::test]
async fn test_sd_flash_windows() {
    // Test Windows-specific SD card handling
}

#[cfg(target_os = "windows")]
#[tokio::test]
async fn test_sd_format_windows() {
    // Test Windows formatting
}
```

### macOS Testing

#### System Requirements

```bash
# Install Xcode Command Line Tools
xcode-select --install

# Install Homebrew (if not already installed)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

#### Permission Setup

macOS may prompt for permissions when accessing devices. Grant the following:

- **Disk Access**: System Preferences → Security & Privacy → Privacy → Full Disk Access
- **Removable Volumes**: Allow access when prompted

#### Running Tests

```bash
# Change to e2e-tests directory
cd e2e-tests

# Run all tests
cargo test --test e2e --all-features

# Run with elevated privileges (if needed)
sudo -E cargo test --test e2e --features sd
```

#### Platform-Specific Test Examples

```rust
#[cfg(target_os = "macos")]
#[tokio::test]
async fn test_sd_flash_macos() {
    // Test macOS-specific disk utilities
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn test_sd_format_macos() {
    // Test macOS formatting
}
```

## Test Categories

### 1. SD Card Flashing Tests

Located in `tests/e2e/sd_flash.rs`

#### Basic Tests
- Uncompressed image flashing
- Compressed (xz) image flashing
- Customization (hostname, WiFi, SSH, etc.)
- Cancellation handling
- SD card formatting

#### Platform-Specific Tests
- Linux: ext4 formatting, udev integration
- Windows: Windows disk API, NTFS handling
- macOS: diskutil integration, APFS/HFS+ handling

#### Stress Tests
- Large image flashing (100+ MB)
- Progress reporting validation
- Concurrent operation handling

### 2. BeagleConnect Freedom Tests

Located in `tests/e2e/bcf_flash.rs`

#### CC1352P7 Tests
- Flashing with verification
- Flashing without verification
- Device enumeration
- Progress reporting
- Platform-specific USB handling

#### MSP430 Tests
- MSP430 firmware flashing
- Device detection
- HID communication
- Platform-specific HID handling

### 3. DFU Flashing Tests

Located in `tests/e2e/dfu_flash.rs`

#### Core Tests
- Single firmware flashing
- Multiple firmware flashing
- Identifier-based device selection
- Cancellation handling

#### Advanced Tests
- Progress reporting
- Invalid firmware error handling
- Device identifier parsing
- Platform-specific USB handling

## Running Tests

### Quick Reference

```bash
# Navigate to e2e-tests directory
cd e2e-tests

# Run all tests
./run_platform_tests.sh --all

# Run specific suites
./run_platform_tests.sh --sd
./run_platform_tests.sh --bcf
./run_platform_tests.sh --dfu

# Generate test report
./run_platform_tests.sh --all --report

# Verbose output
./run_platform_tests.sh --sd --verbose
```

### Using Cargo

```bash
# All tests
cargo test --test e2e --all-features

# Specific features
cargo test --test e2e --features sd
cargo test --test e2e --features bcf,bcf_msp430
cargo test --test e2e --features dfu

# Specific test by name
cargo test --test e2e --features sd test_sd_flash_uncompressed

# Platform-specific tests only
cargo test --test e2e --features sd test_sd_flash_linux
cargo test --test e2e --features sd test_sd_flash_windows
cargo test --test e2e --features sd test_sd_flash_macos

# With verbose output
cargo test --test e2e --features sd -- --nocapture

# Run single test
cargo test --test e2e --features sd test_sd_flash_uncompressed -- --exact
```

### Test Runner Options

The `run_platform_tests.sh` script supports:

- `--sd`: Run SD card tests only
- `--bcf`: Run BCF tests only
- `--dfu`: Run DFU tests only
- `--all`: Run all tests (default)
- `--verbose`: Show verbose output
- `--report`: Generate detailed test report
- `--help`: Show usage information

## CI/CD Integration

### GitHub Actions

A complete GitHub Actions workflow is provided in `.github/workflows/e2e-tests.yml`.

Key features:
- Runs on all three platforms (Linux, Windows, macOS)
- Matrix strategy for parallel execution
- Test report artifacts
- Summary generation

### Running in CI

```yaml
# Example GitHub Actions job
jobs:
  e2e-tests:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - name: Install dependencies (Linux)
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y libudev-dev libusb-1.0-0-dev
      - name: Run tests
        run: |
          cd e2e-tests
          cargo test --test e2e --all-features
```

### Test Reports

Generated reports include:
- Platform information
- Test execution details
- Pass/fail status
- Error messages and stack traces
- Performance metrics

## Hardware Requirements

### Virtual Device Testing (No Hardware)

All test suites support virtual device testing:

- **SD Card**: Uses temporary files as virtual SD cards
- **BCF/DFU**: Skips tests when no device is connected

Perfect for CI/CD and development environments without physical hardware.

### Physical Device Testing

For comprehensive testing with real hardware:

#### SD Card
- Physical SD card (any size)
- SD card reader
- Appropriate permissions (see platform-specific sections)

#### BeagleConnect Freedom
- BeagleConnect Freedom board
- USB cable
- WinUSB drivers (Windows)

#### DFU Devices
- DFU-compatible device
- USB cable
- WinUSB drivers (Windows)

### Device Detection

Tests automatically detect available hardware:

```rust
let destinations = bb_flasher::sd::Target::destinations().await;

if destinations.is_empty() {
    eprintln!("Skipping test: No device found");
    return;
}
```

## Troubleshooting

### Common Issues

#### 1. "No devices found" Warnings

**Cause**: No physical devices connected.

**Solution**: 
- Tests will skip gracefully
- Connect hardware for full testing
- Virtual device tests run without hardware

#### 2. Permission Denied (Linux)

**Cause**: Insufficient permissions to access devices.

**Solutions**:
```bash
# Option 1: Run with sudo
sudo -E cargo test -p e2e-tests --features sd

# Option 2: Set up udev rules (recommended)
sudo usermod -a -G plugdev $USER
# Log out and back in

# Option 3: Use virtual device tests
cargo test -p e2e-tests --features sd
```

#### 3. USB Device Not Detected (Windows)

**Cause**: Missing or incorrect USB drivers.

**Solutions**:
1. Install WinUSB driver using Zadig
2. Verify device appears in Device Manager
3. Reconnect the device
4. Run as administrator

#### 4. macOS Permission Prompts

**Cause**: macOS security restrictions.

**Solutions**:
1. Grant Full Disk Access in System Preferences
2. Allow access when prompted
3. Run with sudo if needed: `sudo -E cargo test ...`

#### 5. Compilation Errors

**Linux**: Missing system libraries
```bash
sudo apt-get install libudev-dev libusb-1.0-0-dev pkg-config
```

**Windows**: Missing Visual Studio Build Tools
- Install Visual Studio with C++ support

**macOS**: Missing Xcode Command Line Tools
```bash
xcode-select --install
```

### Debug Mode

Enable debug logging:

```bash
RUST_LOG=debug cargo test -p e2e-tests --features sd -- --nocapture
```

### Isolated Test Execution

Run single test in isolation:

```bash
cargo test -p e2e-tests --features sd test_sd_flash_uncompressed -- --exact --nocapture
```

## Contributing

### Adding New Tests

1. **Choose the appropriate test file**:
   - `sd_flash.rs` for SD card tests
   - `bcf_flash.rs` for BCF tests
   - `dfu_flash.rs` for DFU tests

2. **Follow the naming convention**:
   ```rust
   // Cross-platform test
   #[tokio::test]
   async fn test_<feature>_<action>() { }
   
   // Platform-specific test
   #[cfg(target_os = "linux")]
   #[tokio::test]
   async fn test_<feature>_<action>_linux() { }
   ```

3. **Use common utilities**:
   ```rust
   use super::common;
   
   let img = common::create_test_image(1024 * 1024)?;
   common::cleanup_test_file(&img)?;
   ```

4. **Handle missing devices gracefully**:
   ```rust
   if destinations.is_empty() {
       eprintln!("Skipping test: No device found");
       return;
   }
   ```

5. **Clean up resources**:
   ```rust
   common::cleanup_test_file(&img_path).ok();
   common::cleanup_test_file(&sd_path).ok();
   ```

### Test Quality Guidelines

- ✅ Add documentation explaining what the test validates
- ✅ Test both success and failure paths
- ✅ Include platform-specific variants when needed
- ✅ Handle cancellation and error cases
- ✅ Verify progress reporting works
- ✅ Clean up temporary files
- ✅ Use meaningful assertion messages

### Updating Documentation

When adding tests, update:
1. This guide (E2E_TESTING_GUIDE.md)
2. Test suite README (e2e-tests/README.md)
3. Main project documentation
4. CI/CD workflows if needed

## Best Practices

### 1. Virtual vs Physical Device Tests

- Use virtual devices for CI/CD
- Use physical devices for comprehensive validation
- Name tests clearly to indicate which is which

### 2. Test Independence

- Each test should be independent
- Don't rely on test execution order
- Clean up all resources

### 3. Error Handling

- Use descriptive error messages
- Include context in assertions
- Log helpful debug information

### 4. Platform Coverage

- Test on all three platforms before merging
- Add platform-specific tests when needed
- Document platform-specific behavior

### 5. Performance

- Use appropriate test image sizes
- Don't create unnecessarily large files
- Clean up promptly to save disk space

## Resources

- [Main Documentation](../docs/E2E_TESTING.md)
- [Test Suite README](../e2e-tests/README.md)
- [CI/CD Workflow](../.github/workflows/e2e-tests.yml)
- [Test Runner Script](../e2e-tests/run_platform_tests.sh)

## License

Same as the parent project. See LICENSE file in the repository root.

