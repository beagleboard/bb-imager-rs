//! Shared helpers for E2E-style integration tests.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Create a test image file with specific size and a simple repeating pattern.
pub fn create_test_image(size: usize) -> std::io::Result<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let img_path = temp_dir.join(format!("test_image_{}.img", uuid::Uuid::new_v4()));

    let mut file = File::create(&img_path)?;

    let pattern: Vec<u8> = (0..256).map(|x| x as u8).collect();
    let chunks = size / pattern.len();
    let remainder = size % pattern.len();

    for _ in 0..chunks {
        file.write_all(&pattern)?;
    }
    if remainder > 0 {
        file.write_all(&pattern[..remainder])?;
    }

    file.flush()?;
    Ok(img_path)
}

/// Create a virtual SD card target for testing backed by a regular file.
pub fn create_virtual_sd_card(size: usize) -> std::io::Result<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let sd_path = temp_dir.join(format!("virtual_sd_{}.img", uuid::Uuid::new_v4()));

    let file = File::create(&sd_path)?;
    file.set_len(size as u64)?;

    Ok(sd_path)
}

/// Cleanup test files, ignoring non-existent paths.
pub fn cleanup_test_file(path: &Path) -> std::io::Result<()> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

