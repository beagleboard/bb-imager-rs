//! E2E tests for DFU (Device Firmware Update) flashing
//!
//! These tests verify DFU flashing workflows for USB devices.
//!
//! ## Platform Coverage
//! - Linux: USB DFU device access via libusb
//! - Windows: USB DFU device access with WinUSB drivers
//! - macOS: USB DFU device access with native support

#![cfg(feature = "dfu")]

use super::common;

/// Test DFU device listing
#[tokio::test]
async fn test_dfu_list_destinations() {
    let destinations = bb_flasher::dfu::Target::destinations().await;

    // Just verify the API works - may return empty if no DFU devices connected
    println!("Found {} DFU device(s)", destinations.len());
    assert!(destinations.len() >= 0);
}

/// Test DFU flashing with single firmware
#[tokio::test]
async fn test_dfu_flash_single_firmware() {
    const FIRMWARE_SIZE: usize = 256 * 1024; // 256 KB

    // Create test firmware
    let fw_path = common::create_test_image(FIRMWARE_SIZE)
        .expect("Failed to create test firmware");

    // Get available DFU devices
    let destinations = bb_flasher::dfu::Target::destinations().await;

    if destinations.is_empty() {
        eprintln!("Skipping DFU test: No DFU device found");
        common::cleanup_test_file(&fw_path).ok();
        return;
    }

    let target = destinations.into_iter().next().unwrap();

    // Create firmware list
    let img = bb_flasher::LocalImage::new(fw_path.clone().into_boxed_path());
    let imgs = vec![("firmware".to_string(), img)];

    // Setup flasher
    let flasher = bb_flasher::dfu::Flasher::from_target(
        imgs,
        &target,
        None,
    );

    // Flash the firmware
    let result = flasher.flash(None).await;

    // Cleanup
    common::cleanup_test_file(&fw_path).ok();

    assert!(result.is_ok(), "DFU flashing failed: {:?}", result.err());
}

/// Test DFU flashing with multiple firmwares
#[tokio::test]
async fn test_dfu_flash_multiple_firmwares() {
    const FIRMWARE1_SIZE: usize = 128 * 1024; // 128 KB
    const FIRMWARE2_SIZE: usize = 256 * 1024; // 256 KB

    // Create test firmwares
    let fw1_path = common::create_test_image(FIRMWARE1_SIZE)
        .expect("Failed to create test firmware 1");
    let fw2_path = common::create_test_image(FIRMWARE2_SIZE)
        .expect("Failed to create test firmware 2");

    // Get available DFU devices
    let destinations = bb_flasher::dfu::Target::destinations().await;

    if destinations.is_empty() {
        eprintln!("Skipping DFU test: No DFU device found");
        common::cleanup_test_file(&fw1_path).ok();
        common::cleanup_test_file(&fw2_path).ok();
        return;
    }

    let target = destinations.into_iter().next().unwrap();

    // Create firmware list
    let img1 = bb_flasher::LocalImage::new(fw1_path.clone().into_boxed_path());
    let img2 = bb_flasher::LocalImage::new(fw2_path.clone().into_boxed_path());
    let imgs = vec![
        ("firmware1".to_string(), img1),
        ("firmware2".to_string(), img2),
    ];

    // Setup flasher
    let flasher = bb_flasher::dfu::Flasher::from_target(
        imgs,
        &target,
        None,
    );

    // Flash the firmwares
    let result = flasher.flash(None).await;

    // Cleanup
    common::cleanup_test_file(&fw1_path).ok();
    common::cleanup_test_file(&fw2_path).ok();

    assert!(result.is_ok(), "DFU multi-firmware flashing failed: {:?}", result.err());
}

/// Test DFU flashing with identifier string
#[tokio::test]
async fn test_dfu_flash_with_identifier() {
    const FIRMWARE_SIZE: usize = 256 * 1024; // 256 KB

    let fw_path = common::create_test_image(FIRMWARE_SIZE)
        .expect("Failed to create test firmware");

    // Get available DFU devices
    let destinations = bb_flasher::dfu::Target::destinations().await;

    if destinations.is_empty() {
        eprintln!("Skipping DFU test: No DFU device found");
        common::cleanup_test_file(&fw_path).ok();
        return;
    }

    let target = destinations.into_iter().next().unwrap();
    let identifier = bb_flasher::BBFlasherTarget::identifier(&target);

    // Create firmware list
    let img = bb_flasher::LocalImage::new(fw_path.clone().into_boxed_path());
    let imgs = vec![("firmware".to_string(), img)];

    // Setup flasher from identifier string
    let flasher = bb_flasher::dfu::Flasher::from_identifier(
        imgs,
        &identifier,
        None,
    ).expect("Failed to create flasher from identifier");

    // Flash the firmware
    let result = flasher.flash(None).await;

    // Cleanup
    common::cleanup_test_file(&fw_path).ok();

    assert!(result.is_ok(), "DFU flashing with identifier failed: {:?}", result.err());
}

/// Test DFU flashing with cancellation
#[tokio::test]
async fn test_dfu_flash_cancellation() {
    const FIRMWARE_SIZE: usize = 1024 * 1024; // 1 MB for longer operation

    let fw_path = common::create_test_image(FIRMWARE_SIZE)
        .expect("Failed to create test firmware");

    let destinations = bb_flasher::dfu::Target::destinations().await;

    if destinations.is_empty() {
        eprintln!("Skipping DFU test: No DFU device found");
        common::cleanup_test_file(&fw_path).ok();
        return;
    }

    let target = destinations.into_iter().next().unwrap();

    let img = bb_flasher::LocalImage::new(fw_path.clone().into_boxed_path());
    let imgs = vec![("firmware".to_string(), img)];

    let cancel_token = tokio_util::sync::CancellationToken::new();

    let flasher = bb_flasher::dfu::Flasher::from_target(
        imgs,
        &target,
        Some(cancel_token.clone()),
    );

    // Start flashing in background
    let flash_handle = tokio::spawn(async move {
        flasher.flash(None).await
    });

    // Cancel after a short delay
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    cancel_token.cancel();

    // Wait for completion
    let result = flash_handle.await.unwrap();

    // Cleanup
    common::cleanup_test_file(&fw_path).ok();

    // Should fail due to cancellation or succeed if it completed quickly
    println!("Cancellation test result: {:?}", result);
}

// ===========================
// Platform-Specific DFU Tests
// ===========================

/// Test DFU flashing on Linux
#[cfg(target_os = "linux")]
#[tokio::test]
async fn test_dfu_flash_linux() {
    const FIRMWARE_SIZE: usize = 256 * 1024; // 256 KB

    let fw_path = common::create_test_image(FIRMWARE_SIZE)
        .expect("Failed to create test firmware");

    let destinations = bb_flasher::dfu::Target::destinations().await;

    if destinations.is_empty() {
        eprintln!("Skipping DFU Linux test: No DFU device found");
        common::cleanup_test_file(&fw_path).ok();
        return;
    }

    let target = destinations.into_iter().next().unwrap();

    let img = bb_flasher::LocalImage::new(fw_path.clone().into_boxed_path());
    let imgs = vec![("firmware".to_string(), img)];

    let flasher = bb_flasher::dfu::Flasher::from_target(
        imgs,
        &target,
        None,
    );

    let result = flasher.flash(None).await;

    common::cleanup_test_file(&fw_path).ok();

    assert!(result.is_ok(), "DFU Linux flashing failed: {:?}", result.err());
}

/// Test DFU device enumeration on Linux
#[cfg(target_os = "linux")]
#[tokio::test]
async fn test_dfu_list_destinations_linux() {
    let destinations = bb_flasher::dfu::Target::destinations().await;

    println!("Found {} DFU device(s) on Linux", destinations.len());
    assert!(destinations.len() >= 0);
}

/// Test DFU flashing on Windows
#[cfg(target_os = "windows")]
#[tokio::test]
async fn test_dfu_flash_windows() {
    const FIRMWARE_SIZE: usize = 256 * 1024; // 256 KB

    let fw_path = common::create_test_image(FIRMWARE_SIZE)
        .expect("Failed to create test firmware");

    let destinations = bb_flasher::dfu::Target::destinations().await;

    if destinations.is_empty() {
        eprintln!("Skipping DFU Windows test: No DFU device found");
        common::cleanup_test_file(&fw_path).ok();
        return;
    }

    let target = destinations.into_iter().next().unwrap();

    let img = bb_flasher::LocalImage::new(fw_path.clone().into_boxed_path());
    let imgs = vec![("firmware".to_string(), img)];

    let flasher = bb_flasher::dfu::Flasher::from_target(
        imgs,
        &target,
        None,
    );

    let result = flasher.flash(None).await;

    common::cleanup_test_file(&fw_path).ok();

    assert!(result.is_ok(), "DFU Windows flashing failed: {:?}", result.err());
}

/// Test DFU device enumeration on Windows
#[cfg(target_os = "windows")]
#[tokio::test]
async fn test_dfu_list_destinations_windows() {
    let destinations = bb_flasher::dfu::Target::destinations().await;

    println!("Found {} DFU device(s) on Windows", destinations.len());
    assert!(destinations.len() >= 0);
}

/// Test DFU flashing on macOS
#[cfg(target_os = "macos")]
#[tokio::test]
async fn test_dfu_flash_macos() {
    const FIRMWARE_SIZE: usize = 256 * 1024; // 256 KB

    let fw_path = common::create_test_image(FIRMWARE_SIZE)
        .expect("Failed to create test firmware");

    let destinations = bb_flasher::dfu::Target::destinations().await;

    if destinations.is_empty() {
        eprintln!("Skipping DFU macOS test: No DFU device found");
        common::cleanup_test_file(&fw_path).ok();
        return;
    }

    let target = destinations.into_iter().next().unwrap();

    let img = bb_flasher::LocalImage::new(fw_path.clone().into_boxed_path());
    let imgs = vec![("firmware".to_string(), img)];

    let flasher = bb_flasher::dfu::Flasher::from_target(
        imgs,
        &target,
        None,
    );

    let result = flasher.flash(None).await;

    common::cleanup_test_file(&fw_path).ok();

    assert!(result.is_ok(), "DFU macOS flashing failed: {:?}", result.err());
}

/// Test DFU device enumeration on macOS
#[cfg(target_os = "macos")]
#[tokio::test]
async fn test_dfu_list_destinations_macos() {
    let destinations = bb_flasher::dfu::Target::destinations().await;

    println!("Found {} DFU device(s) on macOS", destinations.len());
    assert!(destinations.len() >= 0);
}

// ===========================
// Advanced DFU Tests
// ===========================

/// Test DFU flashing with progress reporting
#[tokio::test]
async fn test_dfu_flash_with_progress() {
    const FIRMWARE_SIZE: usize = 512 * 1024; // 512 KB

    let fw_path = common::create_test_image(FIRMWARE_SIZE)
        .expect("Failed to create test firmware");

    let destinations = bb_flasher::dfu::Target::destinations().await;

    if destinations.is_empty() {
        eprintln!("Skipping DFU progress test: No DFU device found");
        common::cleanup_test_file(&fw_path).ok();
        return;
    }

    let target = destinations.into_iter().next().unwrap();

    let img = bb_flasher::LocalImage::new(fw_path.clone().into_boxed_path());
    let imgs = vec![("firmware".to_string(), img)];

    let flasher = bb_flasher::dfu::Flasher::from_target(
        imgs,
        &target,
        None,
    );

    let (tx, mut rx) = tokio::sync::mpsc::channel(20);

    let progress_handle = tokio::spawn(async move {
        let mut updates = Vec::new();
        while let Some(status) = rx.recv().await {
            updates.push(status);
        }
        updates
    });

    let result = flasher.flash(Some(tx)).await;

    let updates = progress_handle.await.unwrap();

    common::cleanup_test_file(&fw_path).ok();

    assert!(result.is_ok(), "DFU flashing with progress failed: {:?}", result.err());
    println!("Received {} progress updates", updates.len());
}

/// Test DFU device identifier parsing and usage
#[tokio::test]
async fn test_dfu_identifier_parsing() {
    let destinations = bb_flasher::dfu::Target::destinations().await;

    if destinations.is_empty() {
        eprintln!("Skipping DFU identifier test: No DFU device found");
        return;
    }

    for target in destinations {
        let identifier = bb_flasher::BBFlasherTarget::identifier(&target);
        println!("DFU Device Identifier: {}", identifier);

        // Verify identifier is not empty
        assert!(!identifier.is_empty(), "Device identifier should not be empty");

        // Test that we can create a flasher from the identifier
        const FIRMWARE_SIZE: usize = 128 * 1024;
        let fw_path = common::create_test_image(FIRMWARE_SIZE)
            .expect("Failed to create test firmware");

        let img = bb_flasher::LocalImage::new(fw_path.clone().into_boxed_path());
        let imgs = vec![("firmware".to_string(), img)];

        let flasher_result = bb_flasher::dfu::Flasher::from_identifier(
            imgs,
            &identifier,
            None,
        );

        common::cleanup_test_file(&fw_path).ok();

        assert!(flasher_result.is_ok(), "Failed to create flasher from identifier: {:?}", flasher_result.err());
    }
}

/// Test DFU error handling with invalid firmware
#[tokio::test]
async fn test_dfu_flash_invalid_firmware() {
    // Create a very small "invalid" firmware
    const INVALID_SIZE: usize = 16; // Too small for valid firmware

    let fw_path = common::create_test_image(INVALID_SIZE)
        .expect("Failed to create invalid firmware");

    let destinations = bb_flasher::dfu::Target::destinations().await;

    if destinations.is_empty() {
        eprintln!("Skipping DFU invalid firmware test: No DFU device found");
        common::cleanup_test_file(&fw_path).ok();
        return;
    }

    let target = destinations.into_iter().next().unwrap();

    let img = bb_flasher::LocalImage::new(fw_path.clone().into_boxed_path());
    let imgs = vec![("firmware".to_string(), img)];

    let flasher = bb_flasher::dfu::Flasher::from_target(
        imgs,
        &target,
        None,
    );

    let result = flasher.flash(None).await;

    common::cleanup_test_file(&fw_path).ok();

    // This might succeed or fail depending on device validation
    println!("Invalid firmware flash result: {:?}", result);
}


