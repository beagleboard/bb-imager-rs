use std::{
    io::{Cursor, Read, Seek},
    sync::mpsc,
};

use bb_flasher_sd::{ContentType, Customization, Destination};
use tempfile::NamedTempFile;

fn test_file(len: usize) -> std::io::Cursor<Box<[u8]>> {
    let data: Vec<u8> = (0..len)
        .map(|x| x % 255)
        .map(|x| u8::try_from(x).unwrap())
        .collect();
    std::io::Cursor::new(data.into())
}

#[test]
fn test_public_flash_with_temp_file() {
    const FILE_LEN: usize = 16 * 1024; // 16 KB
    let dummy_file = test_file(FILE_LEN);
    let expected_bytes = dummy_file.get_ref().clone();

    // 1. Create a named temporary file to serve as our flash destination
    let temp_destination = NamedTempFile::new().expect("Failed to create temp file");
    let dst = Destination::File(temp_destination.path().into());

    // 2. Image Resolver Closure
    let img_data = expected_bytes.clone();
    let img_resolver = move || {
        let reader = Cursor::new(img_data);
        Ok((reader, FILE_LEN as u64))
    };

    // 3. Bmap Resolver Closure (None for this test)
    let bmap_resolver: Option<fn() -> std::io::Result<Box<str>>> = None;

    // 4. Progress Channel
    let (tx, rx) = mpsc::sync_channel(32);

    // 5. Empty Customizations Iterator
    // (Uses a dummy type that fulfills the compiler's expected type constraints)
    let customizations =
        std::iter::empty::<Customization<std::iter::Empty<(Box<str>, ContentType)>>>();

    // 6. Execute the public flash function
    let result = bb_flasher_sd::flash(
        img_resolver,
        bmap_resolver,
        dst,
        Some(tx),
        customizations,
        None,
    );

    assert!(result.is_ok(), "Public flash failed: {:?}", result.err());

    // 7. Verify the contents written to the temporary file
    // Reopen the temp file or persist it to read its contents
    let mut written_file = temp_destination
        .reopen()
        .expect("Failed to reopen temp file");
    let mut written_bytes = Vec::new();

    written_file.rewind().unwrap();
    written_file.read_to_end(&mut written_bytes).unwrap();

    assert_eq!(written_bytes.len(), FILE_LEN);
    assert_eq!(written_bytes, expected_bytes.into_vec());

    // 8. Verify progress track completeness
    let progress_updates: Vec<f32> = rx.try_iter().collect();
    assert!(!progress_updates.is_empty());
    assert_eq!(*progress_updates.last().unwrap(), 1.0);
}
