#![cfg(feature = "mock_sd")]

//! End-to-end test of the public `flash` entry point *with customizations
//! applied*. The existing tests/flashing.rs public test flashes with EMPTY
//! customizations, so the customization loop in `flash` is never exercised
//! through the public API. This drives it by flashing a full (MBR + FAT32)
//! image and asserting the injected file lands in the boot partition.

use std::io::{Cursor, Read};

use bb_flasher_sd::mock_sd::MockSd;
use bb_flasher_sd::{ContentType, Customization, Destination, ParitionType};

#[test]
fn flash_applies_customization_through_public_api() {
    // A freshly-created MockSd is a valid 128 MiB MBR + FAT32 image. Use its
    // bytes as the OS image and flash them back onto its own path so the result
    // can be inspected with `open_boot`.
    let mut mock = MockSd::new();
    let image_bytes: Box<[u8]> = std::fs::read(mock.path()).unwrap().into_boxed_slice();
    let img_size = image_bytes.len() as u64;

    let img_resolver = move || Ok((Cursor::new(image_bytes), img_size));
    let bmap: Option<fn() -> std::io::Result<Box<str>>> = None;

    const FILE_NAME: &str = "customization.txt";
    const FILE_DATA: &[u8] = b"hello from the flasher test";
    // `ContentType` is not Send, so the content iterator must construct it
    // lazily from Send inputs via `map` (a Map iterator is Send when its inner
    // iterator and closure are, regardless of the item type) — the same shape
    // the real facade uses to satisfy `flash`'s `+ Send` bound.
    let content = vec![(FILE_NAME.into(), FILE_DATA.to_vec().into_boxed_slice())]
        .into_iter()
        .map(|(name, data): (Box<str>, Box<[u8]>)| (name, ContentType::DataAppend(data)));
    let customization = Customization {
        partition: ParitionType::Boot,
        content,
    };

    // Progress-to-completion is covered by tests/flashing.rs; this test focuses
    // on the customization loop, so it flashes without a progress channel.
    bb_flasher_sd::flash(
        img_resolver,
        bmap,
        Destination::File(mock.path().into()),
        None,
        std::iter::once(customization),
        None,
    )
    .expect("flash with customization should succeed");

    // The customization file should now exist in the boot partition.
    let fs = mock.open_boot();
    let mut contents = String::new();
    fs.root_dir()
        .open_file(FILE_NAME)
        .expect("customization file should exist in boot partition")
        .read_to_string(&mut contents)
        .unwrap();
    assert_eq!(contents.as_bytes(), FILE_DATA);
}
