//! Common utilities for E2E tests

use std::fs::File;
use std::io::{Seek, Write};
use std::path::{Path, PathBuf};

/// Create a test image file with specific size
pub fn create_test_image(size: usize) -> std::io::Result<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let img_path = temp_dir.join(format!("test_image_{}.img", uuid::Uuid::new_v4()));

    let mut file = File::create(&img_path)?;

    // Create a pattern-based image for verification
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

/// Create a compressed test image (xz format)
pub fn create_compressed_test_image(size: usize) -> std::io::Result<PathBuf> {
    let img_path = create_test_image(size)?;
    let compressed_path = img_path.with_extension("img.xz");

    // Compress using xz2
    let input = std::fs::read(&img_path)?;
    let mut compressor = xz2::write::XzEncoder::new(Vec::new(), 6);
    compressor.write_all(&input)?;
    let compressed = compressor.finish()?;

    std::fs::write(&compressed_path, compressed)?;
    std::fs::remove_file(img_path)?;

    Ok(compressed_path)
}

/// Create a virtual SD card target for testing
pub fn create_virtual_sd_card(size: usize) -> std::io::Result<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let sd_path = temp_dir.join(format!("virtual_sd_{}.img", uuid::Uuid::new_v4()));

    let mut file = File::create(&sd_path)?;
    file.set_len(size as u64)?;
    file.flush()?;

    Ok(sd_path)
}

/// Verify that an image was written correctly
pub fn verify_written_image(written_path: &Path, original_path: &Path) -> std::io::Result<bool> {
    use std::io::Read;

    let mut written = File::open(written_path)?;
    let mut original = File::open(original_path)?;

    let mut written_buf = vec![0u8; 4096];
    let mut original_buf = vec![0u8; 4096];

    loop {
        let written_len = written.read(&mut written_buf)?;
        let original_len = original.read(&mut original_buf)?;

        if written_len != original_len {
            return Ok(false);
        }

        if written_len == 0 {
            break;
        }

        if written_buf[..written_len] != original_buf[..original_len] {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Cleanup test files
pub fn cleanup_test_file(path: &Path) -> std::io::Result<()> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_image() {
        let img = create_test_image(1024).unwrap();
        assert!(img.exists());
        assert_eq!(std::fs::metadata(&img).unwrap().len(), 1024);
        cleanup_test_file(&img).unwrap();
    }

    #[test]
    fn test_create_compressed_image() {
        let img = create_compressed_test_image(1024).unwrap();
        assert!(img.exists());
        assert!(img.to_string_lossy().ends_with(".xz"));
        cleanup_test_file(&img).unwrap();
    }
}

