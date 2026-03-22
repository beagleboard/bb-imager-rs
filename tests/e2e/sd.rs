//! E2E-style tests for SD card flashing.
//!
//! These tests exercise the SD flasher using a virtual file-backed device so
//! they can run in CI without requiring real SD card hardware.

use crate::e2e::{cleanup_test_file, create_test_image, create_virtual_sd_card};

/// Basic SD card flashing with an uncompressed image into a virtual device.
#[tokio::test]
async fn sd_flash_uncompressed_to_virtual_device() {
    const IMAGE_SIZE: usize = 4 * 1024 * 1024; // 4 MB
    const SD_SIZE: usize = 16 * 1024 * 1024; // 16 MB

    let img_path = create_test_image(IMAGE_SIZE).expect("failed to create test image");
    let sd_path = create_virtual_sd_card(SD_SIZE).expect("failed to create virtual SD card");

    let img = bb_flasher::LocalImage::new(img_path.clone().into_boxed_path());
    let target: bb_flasher::sd::Target = sd_path.clone().try_into().expect("failed to create SD target");

    let config = bb_flasher::sd::FlashingSdLinuxConfig::none();

    let flasher = bb_flasher::sd::Flasher::new(
        img,
        None::<bb_helper::resolvable::LocalStringFile>,
        target,
        config,
        None,
    );

    let result = flasher.flash(None).await;

    cleanup_test_file(&img_path).ok();
    cleanup_test_file(&sd_path).ok();

    assert!(result.is_ok(), "flashing uncompressed image failed: {:?}", result.err());
}

/// SD card formatting on a virtual device.
#[tokio::test]
async fn sd_format_virtual_device() {
    const SD_SIZE: usize = 16 * 1024 * 1024; // 16 MB

    let sd_path = create_virtual_sd_card(SD_SIZE).expect("failed to create virtual SD card");

    let target: bb_flasher::sd::Target = sd_path.clone().try_into().expect("failed to create SD target");

    let formatter = bb_flasher::sd::Formatter::new(target, None);

    let result = formatter.format(None).await;

    cleanup_test_file(&sd_path).ok();

    assert!(result.is_ok(), "formatting virtual device failed: {:?}", result.err());
}
