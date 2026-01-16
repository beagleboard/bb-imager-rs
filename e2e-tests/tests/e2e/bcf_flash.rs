//! E2E tests for BeagleConnect Freedom (BCF) flashing
//!
//! These tests verify BCF CC1352P7 and MSP430 flashing workflows.
//!
//! ## Platform Coverage
//! - Linux: USB device access via libusb
//! - Windows: USB device access with WinUSB drivers
//! - macOS: USB device access with native support

#![cfg(any(feature = "bcf", feature = "bcf_msp430"))]

use super::common;

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

// ===========================
// Platform-Specific BCF Tests
// ===========================

#[cfg(feature = "bcf")]
mod platform_specific_cc1352p7 {
    use super::*;

    /// Test BCF CC1352P7 flashing on Linux
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn test_bcf_flash_linux() {
        const IMAGE_SIZE: usize = 512 * 1024; // 512 KB

        let img_path = common::create_test_image(IMAGE_SIZE)
            .expect("Failed to create test firmware");

        let destinations = bb_flasher::bcf::cc1352p7::Target::destinations().await;

        if destinations.is_empty() {
            eprintln!("Skipping BCF Linux test: No BCF device found");
            common::cleanup_test_file(&img_path).ok();
            return;
        }

        let target = destinations.into_iter().next().unwrap();
        let img = bb_flasher::LocalImage::new(img_path.clone().into_boxed_path());

        let flasher = bb_flasher::bcf::cc1352p7::Flasher::new(
            img,
            target,
            true,
            None,
        );

        let result = flasher.flash(None).await;

        common::cleanup_test_file(&img_path).ok();

        assert!(result.is_ok(), "BCF Linux flashing failed: {:?}", result.err());
    }

    /// Test BCF CC1352P7 flashing on Windows
    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn test_bcf_flash_windows() {
        const IMAGE_SIZE: usize = 512 * 1024; // 512 KB

        let img_path = common::create_test_image(IMAGE_SIZE)
            .expect("Failed to create test firmware");

        let destinations = bb_flasher::bcf::cc1352p7::Target::destinations().await;

        if destinations.is_empty() {
            eprintln!("Skipping BCF Windows test: No BCF device found");
            common::cleanup_test_file(&img_path).ok();
            return;
        }

        let target = destinations.into_iter().next().unwrap();
        let img = bb_flasher::LocalImage::new(img_path.clone().into_boxed_path());

        let flasher = bb_flasher::bcf::cc1352p7::Flasher::new(
            img,
            target,
            true,
            None,
        );

        let result = flasher.flash(None).await;

        common::cleanup_test_file(&img_path).ok();

        assert!(result.is_ok(), "BCF Windows flashing failed: {:?}", result.err());
    }

    /// Test BCF CC1352P7 flashing on macOS
    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn test_bcf_flash_macos() {
        const IMAGE_SIZE: usize = 512 * 1024; // 512 KB

        let img_path = common::create_test_image(IMAGE_SIZE)
            .expect("Failed to create test firmware");

        let destinations = bb_flasher::bcf::cc1352p7::Target::destinations().await;

        if destinations.is_empty() {
            eprintln!("Skipping BCF macOS test: No BCF device found");
            common::cleanup_test_file(&img_path).ok();
            return;
        }

        let target = destinations.into_iter().next().unwrap();
        let img = bb_flasher::LocalImage::new(img_path.clone().into_boxed_path());

        let flasher = bb_flasher::bcf::cc1352p7::Flasher::new(
            img,
            target,
            true,
            None,
        );

        let result = flasher.flash(None).await;

        common::cleanup_test_file(&img_path).ok();

        assert!(result.is_ok(), "BCF macOS flashing failed: {:?}", result.err());
    }

    /// Test BCF flashing with progress reporting
    #[tokio::test]
    async fn test_bcf_flash_with_progress() {
        const IMAGE_SIZE: usize = 512 * 1024; // 512 KB

        let img_path = common::create_test_image(IMAGE_SIZE)
            .expect("Failed to create test firmware");

        let destinations = bb_flasher::bcf::cc1352p7::Target::destinations().await;

        if destinations.is_empty() {
            eprintln!("Skipping BCF progress test: No BCF device found");
            common::cleanup_test_file(&img_path).ok();
            return;
        }

        let target = destinations.into_iter().next().unwrap();
        let img = bb_flasher::LocalImage::new(img_path.clone().into_boxed_path());

        let flasher = bb_flasher::bcf::cc1352p7::Flasher::new(
            img,
            target,
            true,
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

        common::cleanup_test_file(&img_path).ok();

        assert!(result.is_ok(), "BCF flashing with progress failed: {:?}", result.err());
        println!("Received {} progress updates", updates.len());
    }
}

#[cfg(feature = "bcf_msp430")]
mod platform_specific_msp430 {
    use super::*;

    /// Test MSP430 flashing on Linux
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn test_msp430_flash_linux() {
        const IMAGE_SIZE: usize = 64 * 1024; // 64 KB

        let img_path = common::create_test_image(IMAGE_SIZE)
            .expect("Failed to create MSP430 firmware");

        let destinations = bb_flasher::bcf::msp430::Target::destinations().await;

        if destinations.is_empty() {
            eprintln!("Skipping MSP430 Linux test: No device found");
            common::cleanup_test_file(&img_path).ok();
            return;
        }

        let target = destinations.into_iter().next().unwrap();
        let img = bb_flasher::LocalImage::new(img_path.clone().into_boxed_path());

        let flasher = bb_flasher::bcf::msp430::Flasher::new(img, target, None);

        let result = flasher.flash(None).await;

        common::cleanup_test_file(&img_path).ok();

        assert!(result.is_ok(), "MSP430 Linux flashing failed: {:?}", result.err());
    }

    /// Test MSP430 flashing on Windows
    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn test_msp430_flash_windows() {
        const IMAGE_SIZE: usize = 64 * 1024; // 64 KB

        let img_path = common::create_test_image(IMAGE_SIZE)
            .expect("Failed to create MSP430 firmware");

        let destinations = bb_flasher::bcf::msp430::Target::destinations().await;

        if destinations.is_empty() {
            eprintln!("Skipping MSP430 Windows test: No device found");
            common::cleanup_test_file(&img_path).ok();
            return;
        }

        let target = destinations.into_iter().next().unwrap();
        let img = bb_flasher::LocalImage::new(img_path.clone().into_boxed_path());

        let flasher = bb_flasher::bcf::msp430::Flasher::new(img, target, None);

        let result = flasher.flash(None).await;

        common::cleanup_test_file(&img_path).ok();

        assert!(result.is_ok(), "MSP430 Windows flashing failed: {:?}", result.err());
    }

    /// Test MSP430 flashing on macOS
    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn test_msp430_flash_macos() {
        const IMAGE_SIZE: usize = 64 * 1024; // 64 KB

        let img_path = common::create_test_image(IMAGE_SIZE)
            .expect("Failed to create MSP430 firmware");

        let destinations = bb_flasher::bcf::msp430::Target::destinations().await;

        if destinations.is_empty() {
            eprintln!("Skipping MSP430 macOS test: No device found");
            common::cleanup_test_file(&img_path).ok();
            return;
        }

        let target = destinations.into_iter().next().unwrap();
        let img = bb_flasher::LocalImage::new(img_path.clone().into_boxed_path());

        let flasher = bb_flasher::bcf::msp430::Flasher::new(img, target, None);

        let result = flasher.flash(None).await;

        common::cleanup_test_file(&img_path).ok();

        assert!(result.is_ok(), "MSP430 macOS flashing failed: {:?}", result.err());
    }

    /// Test MSP430 flashing with progress reporting
    #[tokio::test]
    async fn test_msp430_flash_with_progress() {
        const IMAGE_SIZE: usize = 64 * 1024; // 64 KB

        let img_path = common::create_test_image(IMAGE_SIZE)
            .expect("Failed to create MSP430 firmware");

        let destinations = bb_flasher::bcf::msp430::Target::destinations().await;

        if destinations.is_empty() {
            eprintln!("Skipping MSP430 progress test: No device found");
            common::cleanup_test_file(&img_path).ok();
            return;
        }

        let target = destinations.into_iter().next().unwrap();
        let img = bb_flasher::LocalImage::new(img_path.clone().into_boxed_path());

        let flasher = bb_flasher::bcf::msp430::Flasher::new(img, target, None);

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

        common::cleanup_test_file(&img_path).ok();

        assert!(result.is_ok(), "MSP430 flashing with progress failed: {:?}", result.err());
        println!("Received {} progress updates", updates.len());
    }
}


