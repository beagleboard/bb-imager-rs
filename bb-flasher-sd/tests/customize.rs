#![cfg(feature = "mock_sd")]

//! End-to-end tests of the public `flash` entry point *with customizations
//! applied*. The existing tests/flashing.rs public test flashes with EMPTY
//! customizations, so the customization loop in `flash` is never exercised
//! through the public API. These drive it by flashing a full image (MBR + FAT32
//! and GPT + FAT32) and asserting the injected files land in the boot
//! partition.

use std::io::{Cursor, Read, Seek, Write};

use bb_flasher_sd::flashing::NO_BOOTFS;
use bb_flasher_sd::mock_sd::{MockArchive, MockContent, MockSd};
use bb_flasher_sd::{ContentType, Customization, Destination, ParitionType};
use tempfile::NamedTempFile;

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
        NO_BOOTFS,
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

/// GPT analog of the test above: the image is a GPT-partitioned disk, so
/// customization has to find the boot partition through `gptman` instead of the
/// MBR table.
#[test]
fn flash_applies_customization_to_gpt_image() {
    let mut mock = MockSd::new_gpt();
    let mut img = mock.image_copy();
    img.rewind().unwrap();
    let img_size = mock.size();

    const FILE_NAME: &str = "gpt-customization.txt";
    const FILE_DATA: &[u8] = b"hello from the gpt flasher test";
    let content = vec![(FILE_NAME.into(), FILE_DATA.to_vec().into_boxed_slice())]
        .into_iter()
        .map(|(name, data): (Box<str>, Box<[u8]>)| (name, ContentType::DataAppend(data)));
    let customization = Customization {
        partition: ParitionType::Boot,
        content,
    };

    let bmap: Option<fn() -> std::io::Result<Box<str>>> = None;
    bb_flasher_sd::flash(
        move || Ok((img, img_size)),
        NO_BOOTFS,
        bmap,
        Destination::File(mock.path().into()),
        None,
        std::iter::once(customization),
        None,
    )
    .expect("flash with customization should succeed on a GPT image");

    assert_eq!(mock.boot_file(FILE_NAME).unwrap().as_bytes(), FILE_DATA);
}

/// Every [`ContentType`] variant against a GPT disk. Driven through
/// `bootfs_update::flash`, which customizes an already flashed card, so no image
/// write is needed here.
#[test]
fn gpt_boot_partition_accepts_all_content_types() {
    const DIR: &str = "config_dir";
    const READER_PATH: &str = "config_dir/reader.txt";
    const READER_CONTENTS: &str = "reader";
    const APPEND_CONTENTS: &str = "append";
    const FILE_PATH: &str = "config_dir/file.txt";
    const FILE_CONTENTS: &str = "contents pulled in from a path";

    let mut mock = MockSd::new_gpt();

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(FILE_CONTENTS.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let archive = MockArchive::from_entries(vec![
        (DIR.into(), MockContent::Dir),
        (
            READER_PATH.into(),
            MockContent::Reader(READER_CONTENTS.as_bytes().into()),
        ),
        (FILE_PATH.into(), MockContent::File(temp_file.path().into())),
        // Same path as the reader above on purpose: appends onto the entry
        // created earlier in this same archive.
        (
            READER_PATH.into(),
            MockContent::DataAppend(APPEND_CONTENTS.as_bytes().into()),
        ),
    ]);

    bb_flasher_sd::bootfs_update::flash(
        move || Ok(archive),
        Destination::File(mock.path().into()),
        None,
    )
    .expect("bootfs update should succeed on a GPT image");

    {
        let fs = mock.open_boot();
        fs.root_dir()
            .open_dir(DIR)
            .expect("directory entry should exist in boot partition");
    }

    assert_eq!(
        mock.boot_file(READER_PATH).unwrap(),
        format!("{READER_CONTENTS}{APPEND_CONTENTS}")
    );
    assert_eq!(mock.boot_file(FILE_PATH).unwrap(), FILE_CONTENTS);
}

/// GPT's `ending_lba` is inclusive, so the boot partition slice must reach the
/// end of the last sector. Filling the filesystem allocates every cluster,
/// including the one that sits at the very end of the partition.
#[test]
fn gpt_boot_partition_spans_the_last_sector() {
    let mut mock = MockSd::new_gpt();

    let free = {
        let fs = mock.open_boot();
        let stats = fs.stats().unwrap();
        u64::from(stats.free_clusters()) * u64::from(stats.cluster_size())
    };

    let fs = mock.open_boot();
    let root = fs.root_dir();
    let mut f = root.create_file("fill.bin").unwrap();

    const FILL_ERR: &str = "writing up to the end of the boot partition should succeed";
    let chunk = vec![0xa5u8; 64 * 1024];
    let mut written = 0u64;
    while written < free {
        let len = std::cmp::min(chunk.len() as u64, free - written) as usize;
        f.write_all(&chunk[..len]).expect(FILL_ERR);
        written += len as u64;
    }
    f.flush().expect(FILL_ERR);
}
