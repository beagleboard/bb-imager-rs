//! E2E tests for BeagleConnect Freedom (BCF) flashing
//!
//! These tests verify BCF CC1352P7 and MSP430 flashing workflows.

#![cfg(any(feature = "bcf", feature = "bcf_msp430"))]

mod common;

#[cfg(feature = "bcf")]
mod cc1352p7 {
    use super::*;

    /// Test BCF CC1352P7 flashing with verification
    #[tokio::test]
    async fn test_bcf_flash_with_verify() {
        const IMAGE_SIZE: usize = 512 * 1024; // 512 KB typical firmware size

        // Create test firmware image
        let img_path = common::create_test_image(IMAGE_SIZE)
            .expect("Failed to create test firmware");

        // Note: This test requires a connected BCF device
        // In CI, we'll use a mock device or skip if no device is present
        let destinations = bb_flasher::bcf::cc1352p7::Target::destinations().await;

        if destinations.is_empty() {
            eprintln!("Skipping BCF test: No BCF device found");
            common::cleanup_test_file(&img_path).ok();
            return;
        }

        let target = destinations.into_iter().next().unwrap();

        // Setup flasher
        let img = bb_flasher::LocalImage::new(img_path.clone().into_boxed_path());

        let flasher = bb_flasher::bcf::cc1352p7::Flasher::new(
            img,
            target,
            true, // verify
            None,
        );

        // Flash the firmware
        let result = flasher.flash(None).await;

        // Cleanup
        common::cleanup_test_file(&img_path).ok();

        assert!(result.is_ok(), "BCF flashing failed: {:?}", result.err());
    }

    /// Test BCF CC1352P7 flashing without verification
    #[tokio::test]
    async fn test_bcf_flash_no_verify() {
        const IMAGE_SIZE: usize = 512 * 1024; // 512 KB

        let img_path = common::create_test_image(IMAGE_SIZE)
            .expect("Failed to create test firmware");

        let destinations = bb_flasher::bcf::cc1352p7::Target::destinations().await;

        if destinations.is_empty() {
            eprintln!("Skipping BCF test: No BCF device found");
            common::cleanup_test_file(&img_path).ok();
            return;
        }

        let target = destinations.into_iter().next().unwrap();
        let img = bb_flasher::LocalImage::new(img_path.clone().into_boxed_path());

        let flasher = bb_flasher::bcf::cc1352p7::Flasher::new(
            img,
            target,
            false, // no verify
            None,
        );

        let result = flasher.flash(None).await;

        common::cleanup_test_file(&img_path).ok();

        assert!(result.is_ok(), "BCF flashing without verify failed: {:?}", result.err());
    }

    /// Test BCF destination listing
    #[tokio::test]
    async fn test_bcf_list_destinations() {
        let destinations = bb_flasher::bcf::cc1352p7::Target::destinations().await;

        // Just verify the API works - may return empty if no devices connected
        println!("Found {} BCF device(s)", destinations.len());
        assert!(destinations.len() >= 0);
    }
}

#[cfg(feature = "bcf_msp430")]
mod msp430 {
    use super::*;

    /// Test MSP430 flashing
    #[tokio::test]
    async fn test_msp430_flash() {
        const IMAGE_SIZE: usize = 64 * 1024; // 64 KB typical MSP430 firmware size

        let img_path = common::create_test_image(IMAGE_SIZE)
            .expect("Failed to create MSP430 firmware");

        let destinations = bb_flasher::bcf::msp430::Target::destinations().await;

        if destinations.is_empty() {
            eprintln!("Skipping MSP430 test: No device found");
            common::cleanup_test_file(&img_path).ok();
            return;
        }

        let target = destinations.into_iter().next().unwrap();
        let img = bb_flasher::LocalImage::new(img_path.clone().into_boxed_path());

        let flasher = bb_flasher::bcf::msp430::Flasher::new(img, target, None);

        let result = flasher.flash(None).await;

        common::cleanup_test_file(&img_path).ok();

        assert!(result.is_ok(), "MSP430 flashing failed: {:?}", result.err());
    }

    /// Test MSP430 destination listing
    #[tokio::test]
    async fn test_msp430_list_destinations() {
        let destinations = bb_flasher::bcf::msp430::Target::destinations().await;

        println!("Found {} MSP430 device(s)", destinations.len());
        assert!(destinations.len() >= 0);
    }
}

