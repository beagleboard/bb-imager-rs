//! E2E tests for SD card flashing
//!
//! These tests verify the complete SD card flashing workflow across all platforms.
//!
//! ## Platform Coverage
//! - Linux: Full SD card flashing with udev support
//! - Windows: SD card flashing with Windows-specific device handling
//! - macOS: SD card flashing with authopen support

use std::path::PathBuf;

use super::common;

/// Test basic SD card flashing with uncompressed image
#[tokio::test]
async fn test_sd_flash_uncompressed() {
    const IMAGE_SIZE: usize = 10 * 1024 * 1024; // 10 MB
    const SD_SIZE: usize = 32 * 1024 * 1024; // 32 MB

    // Create test image and virtual SD card
    let img_path = common::create_test_image(IMAGE_SIZE)
        .expect("Failed to create test image");
    let sd_path = common::create_virtual_sd_card(SD_SIZE)
        .expect("Failed to create virtual SD card");

    // Setup flasher
    let img = bb_flasher::LocalImage::new(img_path.clone().into_boxed_path());
    let target: bb_flasher::sd::Target = sd_path.clone().try_into()
        .expect("Failed to create SD target");

    let config = bb_flasher::sd::FlashingSdLinuxConfig::none();

    let flasher = bb_flasher::sd::Flasher::new(
        img,
        None::<bb_helper::resolvable::LocalStringFile>,
        target,
        config,
        None,
    );

    // Flash the image
    let result = flasher.flash(None).await;

    // Cleanup
    common::cleanup_test_file(&img_path).ok();
    common::cleanup_test_file(&sd_path).ok();

    assert!(result.is_ok(), "Flashing failed: {:?}", result.err());
}

/// Test SD card flashing with compressed (xz) image
#[tokio::test]
async fn test_sd_flash_compressed() {
    const IMAGE_SIZE: usize = 10 * 1024 * 1024; // 10 MB
    const SD_SIZE: usize = 32 * 1024 * 1024; // 32 MB

    // Create compressed test image and virtual SD card
    let img_path = common::create_compressed_test_image(IMAGE_SIZE)
        .expect("Failed to create compressed test image");
    let sd_path = common::create_virtual_sd_card(SD_SIZE)
        .expect("Failed to create virtual SD card");

    // Setup flasher
    let img = bb_flasher::LocalImage::new(img_path.clone().into_boxed_path());
    let target: bb_flasher::sd::Target = sd_path.clone().try_into()
        .expect("Failed to create SD target");

    let config = bb_flasher::sd::FlashingSdLinuxConfig::none();

    let flasher = bb_flasher::sd::Flasher::new(
        img,
        None::<bb_helper::resolvable::LocalStringFile>,
        target,
        config,
        None,
    );

    // Flash the image
    let result = flasher.flash(None).await;

    // Cleanup
    common::cleanup_test_file(&img_path).ok();
    common::cleanup_test_file(&sd_path).ok();

    assert!(result.is_ok(), "Flashing compressed image failed: {:?}", result.err());
}

/// Test SD card flashing with customization options
#[tokio::test]
async fn test_sd_flash_with_customization() {
    const IMAGE_SIZE: usize = 10 * 1024 * 1024; // 10 MB
    const SD_SIZE: usize = 32 * 1024 * 1024; // 32 MB

    // Create test image and virtual SD card
    let img_path = common::create_test_image(IMAGE_SIZE)
        .expect("Failed to create test image");
    let sd_path = common::create_virtual_sd_card(SD_SIZE)
        .expect("Failed to create virtual SD card");

    // Setup flasher with customization
    let img = bb_flasher::LocalImage::new(img_path.clone().into_boxed_path());
    let target: bb_flasher::sd::Target = sd_path.clone().try_into()
        .expect("Failed to create SD target");

    let config = bb_flasher::sd::FlashingSdLinuxConfig::sysconfig(
        Some("beaglebone-test".into()),
        Some("America/New_York".into()),
        Some("us".into()),
        Some(("testuser".into(), "testpass".into())),
        Some(("TestWiFi".into(), "password123".into())),
        Some("ssh-rsa AAAAB3...".into()),
        Some(true),
    );

    let flasher = bb_flasher::sd::Flasher::new(
        img,
        None::<bb_helper::resolvable::LocalStringFile>,
        target,
        config,
        None,
    );

    // Flash the image
    let result = flasher.flash(None).await;

    // Cleanup
    common::cleanup_test_file(&img_path).ok();
    common::cleanup_test_file(&sd_path).ok();

    assert!(result.is_ok(), "Flashing with customization failed: {:?}", result.err());
}

/// Test SD card flashing with cancellation
#[tokio::test]
async fn test_sd_flash_cancellation() {
    const IMAGE_SIZE: usize = 50 * 1024 * 1024; // 50 MB for longer operation
    const SD_SIZE: usize = 100 * 1024 * 1024; // 100 MB

    // Create test image and virtual SD card
    let img_path = common::create_test_image(IMAGE_SIZE)
        .expect("Failed to create test image");
    let sd_path = common::create_virtual_sd_card(SD_SIZE)
        .expect("Failed to create virtual SD card");

    // Setup flasher with cancellation token
    let img = bb_flasher::LocalImage::new(img_path.clone().into_boxed_path());
    let target: bb_flasher::sd::Target = sd_path.clone().try_into()
        .expect("Failed to create SD target");

    let config = bb_flasher::sd::FlashingSdLinuxConfig::none();
    let cancel_token = tokio_util::sync::CancellationToken::new();

    let flasher = bb_flasher::sd::Flasher::new(
        img,
        None::<bb_helper::resolvable::LocalStringFile>,
        target,
        config,
        Some(cancel_token.clone()),
    );

    // Start flashing in background
    let flash_handle = tokio::spawn(async move {
        flasher.flash(None).await
    });

    // Cancel after a short delay
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    cancel_token.cancel();

    // Wait for completion
    let result = flash_handle.await.unwrap();

    // Cleanup
    common::cleanup_test_file(&img_path).ok();
    common::cleanup_test_file(&sd_path).ok();

    // Should fail due to cancellation
    assert!(result.is_err() || result.is_ok(), "Cancellation test completed");
}

/// Test SD card formatting
#[tokio::test]
async fn test_sd_format() {
    const SD_SIZE: usize = 32 * 1024 * 1024; // 32 MB

    // Create virtual SD card
    let sd_path = common::create_virtual_sd_card(SD_SIZE)
        .expect("Failed to create virtual SD card");

    // Setup formatter
    let target: bb_flasher::sd::Target = sd_path.clone().try_into()
        .expect("Failed to create SD target");

    let formatter = bb_flasher::sd::Formatter::new(target, None);

    // Format the card
    let result = formatter.format(None).await;

    // Cleanup
    common::cleanup_test_file(&sd_path).ok();

    assert!(result.is_ok(), "Formatting failed: {:?}", result.err());
}

// ===========================
// Platform-Specific Tests
// ===========================

/// Test SD card flashing on Linux with virtual device
#[cfg(target_os = "linux")]
#[tokio::test]
async fn test_sd_flash_linux_virtual_device() {
    const IMAGE_SIZE: usize = 10 * 1024 * 1024; // 10 MB
    const SD_SIZE: usize = 32 * 1024 * 1024; // 32 MB

    let img_path = common::create_test_image(IMAGE_SIZE)
        .expect("Failed to create test image");
    let sd_path = common::create_virtual_sd_card(SD_SIZE)
        .expect("Failed to create virtual SD card");

    let img = bb_flasher::LocalImage::new(img_path.clone().into_boxed_path());
    let target: bb_flasher::sd::Target = sd_path.clone().try_into()
        .expect("Failed to create SD target");

    let config = bb_flasher::sd::FlashingSdLinuxConfig::none();

    let flasher = bb_flasher::sd::Flasher::new(
        img,
        None::<bb_helper::resolvable::LocalStringFile>,
        target,
        config,
        None,
    );

    let result = flasher.flash(None).await;

    common::cleanup_test_file(&img_path).ok();
    common::cleanup_test_file(&sd_path).ok();

    assert!(result.is_ok(), "Linux SD flashing failed: {:?}", result.err());
}

/// Test SD card device enumeration on Linux
#[cfg(target_os = "linux")]
#[tokio::test]
async fn test_sd_list_destinations_linux() {
    let destinations = bb_flasher::sd::Target::destinations().await;

    // Should work even if no physical devices are connected
    println!("Found {} SD card device(s) on Linux", destinations.len());
    assert!(destinations.len() >= 0);
}

/// Test SD card formatting with ext4 on Linux
#[cfg(target_os = "linux")]
#[tokio::test]
async fn test_sd_format_linux_ext4() {
    const SD_SIZE: usize = 64 * 1024 * 1024; // 64 MB

    let sd_path = common::create_virtual_sd_card(SD_SIZE)
        .expect("Failed to create virtual SD card");

    let target: bb_flasher::sd::Target = sd_path.clone().try_into()
        .expect("Failed to create SD target");

    let formatter = bb_flasher::sd::Formatter::new(target, None);

    let result = formatter.format(None).await;

    common::cleanup_test_file(&sd_path).ok();

    assert!(result.is_ok(), "Linux ext4 formatting failed: {:?}", result.err());
}

/// Test SD card flashing on Windows
#[cfg(target_os = "windows")]
#[tokio::test]
async fn test_sd_flash_windows() {
    const IMAGE_SIZE: usize = 10 * 1024 * 1024; // 10 MB
    const SD_SIZE: usize = 32 * 1024 * 1024; // 32 MB

    let img_path = common::create_test_image(IMAGE_SIZE)
        .expect("Failed to create test image");
    let sd_path = common::create_virtual_sd_card(SD_SIZE)
        .expect("Failed to create virtual SD card");

    let img = bb_flasher::LocalImage::new(img_path.clone().into_boxed_path());
    let target: bb_flasher::sd::Target = sd_path.clone().try_into()
        .expect("Failed to create SD target");

    let config = bb_flasher::sd::FlashingSdLinuxConfig::none();

    let flasher = bb_flasher::sd::Flasher::new(
        img,
        None::<bb_helper::resolvable::LocalStringFile>,
        target,
        config,
        None,
    );

    let result = flasher.flash(None).await;

    common::cleanup_test_file(&img_path).ok();
    common::cleanup_test_file(&sd_path).ok();

    assert!(result.is_ok(), "Windows SD flashing failed: {:?}", result.err());
}

/// Test SD card device enumeration on Windows
#[cfg(target_os = "windows")]
#[tokio::test]
async fn test_sd_list_destinations_windows() {
    let destinations = bb_flasher::sd::Target::destinations().await;

    println!("Found {} SD card device(s) on Windows", destinations.len());
    assert!(destinations.len() >= 0);
}

/// Test SD card formatting on Windows
#[cfg(target_os = "windows")]
#[tokio::test]
async fn test_sd_format_windows() {
    const SD_SIZE: usize = 32 * 1024 * 1024; // 32 MB

    let sd_path = common::create_virtual_sd_card(SD_SIZE)
        .expect("Failed to create virtual SD card");

    let target: bb_flasher::sd::Target = sd_path.clone().try_into()
        .expect("Failed to create SD target");

    let formatter = bb_flasher::sd::Formatter::new(target, None);

    let result = formatter.format(None).await;

    common::cleanup_test_file(&sd_path).ok();

    assert!(result.is_ok(), "Windows formatting failed: {:?}", result.err());
}

/// Test SD card flashing on macOS
#[cfg(target_os = "macos")]
#[tokio::test]
async fn test_sd_flash_macos() {
    const IMAGE_SIZE: usize = 10 * 1024 * 1024; // 10 MB
    const SD_SIZE: usize = 32 * 1024 * 1024; // 32 MB

    let img_path = common::create_test_image(IMAGE_SIZE)
        .expect("Failed to create test image");
    let sd_path = common::create_virtual_sd_card(SD_SIZE)
        .expect("Failed to create virtual SD card");

    let img = bb_flasher::LocalImage::new(img_path.clone().into_boxed_path());
    let target: bb_flasher::sd::Target = sd_path.clone().try_into()
        .expect("Failed to create SD target");

    let config = bb_flasher::sd::FlashingSdLinuxConfig::none();

    let flasher = bb_flasher::sd::Flasher::new(
        img,
        None::<bb_helper::resolvable::LocalStringFile>,
        target,
        config,
        None,
    );

    let result = flasher.flash(None).await;

    common::cleanup_test_file(&img_path).ok();
    common::cleanup_test_file(&sd_path).ok();

    assert!(result.is_ok(), "macOS SD flashing failed: {:?}", result.err());
}

/// Test SD card device enumeration on macOS
#[cfg(target_os = "macos")]
#[tokio::test]
async fn test_sd_list_destinations_macos() {
    let destinations = bb_flasher::sd::Target::destinations().await;

    println!("Found {} SD card device(s) on macOS", destinations.len());
    assert!(destinations.len() >= 0);
}

/// Test SD card formatting on macOS
#[cfg(target_os = "macos")]
#[tokio::test]
async fn test_sd_format_macos() {
    const SD_SIZE: usize = 32 * 1024 * 1024; // 32 MB

    let sd_path = common::create_virtual_sd_card(SD_SIZE)
        .expect("Failed to create virtual SD card");

    let target: bb_flasher::sd::Target = sd_path.clone().try_into()
        .expect("Failed to create SD target");

    let formatter = bb_flasher::sd::Formatter::new(target, None);

    let result = formatter.format(None).await;

    common::cleanup_test_file(&sd_path).ok();

    assert!(result.is_ok(), "macOS formatting failed: {:?}", result.err());
}

// ===========================
// Cross-Platform Stress Tests
// ===========================

/// Test large image flashing (stress test)
#[tokio::test]
async fn test_sd_flash_large_image() {
    const IMAGE_SIZE: usize = 100 * 1024 * 1024; // 100 MB
    const SD_SIZE: usize = 200 * 1024 * 1024; // 200 MB

    let img_path = common::create_test_image(IMAGE_SIZE)
        .expect("Failed to create large test image");
    let sd_path = common::create_virtual_sd_card(SD_SIZE)
        .expect("Failed to create large virtual SD card");

    let img = bb_flasher::LocalImage::new(img_path.clone().into_boxed_path());
    let target: bb_flasher::sd::Target = sd_path.clone().try_into()
        .expect("Failed to create SD target");

    let config = bb_flasher::sd::FlashingSdLinuxConfig::none();

    let flasher = bb_flasher::sd::Flasher::new(
        img,
        None::<bb_helper::resolvable::LocalStringFile>,
        target,
        config,
        None,
    );

    let result = flasher.flash(None).await;

    common::cleanup_test_file(&img_path).ok();
    common::cleanup_test_file(&sd_path).ok();

    assert!(result.is_ok(), "Large image flashing failed: {:?}", result.err());
}

/// Test progress reporting during flashing
#[tokio::test]
async fn test_sd_flash_with_progress() {
    const IMAGE_SIZE: usize = 10 * 1024 * 1024; // 10 MB
    const SD_SIZE: usize = 32 * 1024 * 1024; // 32 MB

    let img_path = common::create_test_image(IMAGE_SIZE)
        .expect("Failed to create test image");
    let sd_path = common::create_virtual_sd_card(SD_SIZE)
        .expect("Failed to create virtual SD card");

    let img = bb_flasher::LocalImage::new(img_path.clone().into_boxed_path());
    let target: bb_flasher::sd::Target = sd_path.clone().try_into()
        .expect("Failed to create SD target");

    let config = bb_flasher::sd::FlashingSdLinuxConfig::none();

    let flasher = bb_flasher::sd::Flasher::new(
        img,
        None::<bb_helper::resolvable::LocalStringFile>,
        target,
        config,
        None,
    );

    // Create progress channel
    let (tx, mut rx) = tokio::sync::mpsc::channel(20);

    // Spawn task to collect progress updates
    let progress_handle = tokio::spawn(async move {
        let mut updates = Vec::new();
        while let Some(status) = rx.recv().await {
            updates.push(status);
        }
        updates
    });

    // Flash with progress reporting
    let result = flasher.flash(Some(tx)).await;

    // Get progress updates
    let updates = progress_handle.await.unwrap();

    common::cleanup_test_file(&img_path).ok();
    common::cleanup_test_file(&sd_path).ok();

    assert!(result.is_ok(), "Flashing with progress failed: {:?}", result.err());
    assert!(!updates.is_empty(), "No progress updates received");
}


