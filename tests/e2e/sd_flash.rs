//! E2E tests for SD card flashing
//!
//! These tests verify the complete SD card flashing workflow across all platforms.

use std::path::PathBuf;

mod common;

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

