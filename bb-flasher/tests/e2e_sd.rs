//! E2E-style tests for SD card flashing.
//!
//! Why this is skipped by default:
//! - `bb_flasher::sd::Target` only accepts *real removable devices* discovered by
//!   `bb_flasher_sd::devices()`. A temp file created in `/tmp` will never be treated
//!   as a removable disk, so trying to flash/format a file-backed "virtual SD" won't work.
//!
//! If you want to run SD E2E locally on Linux, you can:
//! 1) attach a real removable SD card, or
//! 2) (advanced) create a loop device that shows up as removable (not currently ensured).
//!
//! In CI, this test is ignored so it won't brick disks or fail on non-removable targets.

mod e2e_common;

use bb_flasher::BBFlasher;
use e2e_common::{cleanup_test_file, create_test_image};
use std::convert::TryInto;

/// Placeholder SD E2E test.
///
/// This test is marked `#[ignore]` because safe, portable SD E2E requires a real removable
/// device (or a dedicated test-only backend) which we don't have yet.
#[tokio::test]
#[ignore]
async fn sd_flash_requires_real_device() {
    const IMAGE_SIZE: usize = 4 * 1024 * 1024; // 4 MB

    let img_path = create_test_image(IMAGE_SIZE).expect("failed to create test image");

    // If you want to run this locally, set BB_IMAGER_SD_DEVICE to something like:
    //   /dev/sdb
    // WARNING: THIS WILL ERASE THE DEVICE.
    let dev = std::env::var("BB_IMAGER_SD_DEVICE").expect(
        "set BB_IMAGER_SD_DEVICE=/dev/<disk> to run this test (WARNING: destructive)",
    );

    let img = bb_flasher::LocalImage::new(img_path.clone().into_boxed_path());
    let dev_path = std::path::PathBuf::from(dev);
    let target: bb_flasher::sd::Target = dev_path
        .try_into()
        .expect("failed to create SD target (must be a discovered removable device)");

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

    assert!(result.is_ok(), "flashing image failed: {:?}", result.err());
}
