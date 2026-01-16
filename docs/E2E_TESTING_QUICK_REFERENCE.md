# E2E Testing Quick Reference

## Quick Commands

### Run All Tests
```bash
cd e2e-tests
./run_platform_tests.sh --all
```

### Run Specific Test Suite
```bash
# SD Card tests
./run_platform_tests.sh --sd

# BCF tests  
./run_platform_tests.sh --bcf

# DFU tests
./run_platform_tests.sh --dfu

# Multiple suites
./run_platform_tests.sh --sd --dfu
```

### Generate Test Report
```bash
./run_platform_tests.sh --all --report
```

### Verbose Output
```bash
./run_platform_tests.sh --sd --verbose
```

## Cargo Commands

### All Features
```bash
cargo test --test e2e --all-features
```

### Specific Features
```bash
cargo test --test e2e --features sd
cargo test --test e2e --features bcf,bcf_msp430
cargo test --test e2e --features dfu
```

### Single Test
```bash
cargo test --test e2e --features sd test_sd_flash_uncompressed -- --exact
```

### With Output
```bash
cargo test --test e2e --features sd -- --nocapture
```

### Platform-Specific
```bash
# Linux only
cargo test --test e2e --features sd test_sd_flash_linux

# Windows only
cargo test --test e2e --features sd test_sd_flash_windows

# macOS only
cargo test --test e2e --features sd test_sd_flash_macos
```

## Platform Setup

### Linux
```bash
sudo apt-get install libudev-dev libusb-1.0-0-dev pkg-config
sudo usermod -a -G plugdev $USER
```

### Windows
- Install Visual Studio Build Tools
- Install WinUSB drivers using Zadig

### macOS
```bash
xcode-select --install
```

## Test Structure

```
e2e-tests/tests/e2e/
├── common.rs      # Shared utilities
├── sd_flash.rs    # SD card tests (16 tests)
├── bcf_flash.rs   # BCF tests (11 tests)
└── dfu_flash.rs   # DFU tests (12 tests)
```

## Common Issues

### "No devices found"
✅ Normal - tests skip when hardware not connected

### Permission denied (Linux)
```bash
sudo -E cargo test --test e2e --features sd
```

### USB device not detected (Windows)
- Use Zadig to install WinUSB driver
- Run as Administrator

### macOS permissions
- Grant Full Disk Access in System Preferences
- Run with sudo if needed

## Debug Mode
```bash
RUST_LOG=debug cargo test --test e2e --features sd -- --nocapture
```

## Documentation

- Full Guide: `docs/E2E_TESTING_GUIDE.md`
- Test README: `e2e-tests/README.md`
- Summary: `E2E_TESTING_IMPLEMENTATION_SUMMARY.md`

## Test Count by Platform

| Platform | Tests |
|----------|-------|
| Linux    | 7 platform-specific + 18 cross-platform |
| Windows  | 7 platform-specific + 18 cross-platform |
| macOS    | 7 platform-specific + 18 cross-platform |
| **Total** | **39+ tests** |

## CI/CD

GitHub Actions workflow: `.github/workflows/e2e-tests.yml`

Runs automatically on:
- Push to main/develop
- Pull requests
- Manual trigger

## Adding New Tests

1. Choose file: `sd_flash.rs`, `bcf_flash.rs`, or `dfu_flash.rs`
2. Name: `test_<feature>_<action>_<platform>`
3. Use utilities from `common.rs`
4. Handle missing devices gracefully
5. Clean up resources
6. Update documentation

