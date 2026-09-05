#![cfg(feature = "mock_sd")]

//! End-to-end tests of the public `flash` entry point's `bootfs` argument.
//!
//! `tests/bootfs_update.rs` drives `bootfs_update::flash`, which updates the
//! BOOT partition of an *already flashed* card. These tests instead cover the
//! bootfs step folded into `flash` itself: the archive has to be applied to the
//! partition table that the image write just laid down, after the image and
//! before customizations.

use std::io::{self, Read, Seek, Write};
use std::path::Path;

use bb_flasher_sd::mock_sd::{MockArchive, MockContent, MockSd};
use bb_flasher_sd::{ContentType, Customization, Destination, ParitionType};

const DIR: &str = "bootfs_dir";
const READER_PATH: &str = "bootfs_dir/reader.txt";
const READER_CONTENTS: &str = "written by the bootfs archive";
const FILE_PATH: &str = "bootfs_dir/file.txt";

/// BOOT partition entries for the `bootfs` argument. `file` is optional: the
/// ordering test only needs the reader entry.
fn archive(file: Option<Box<Path>>) -> MockArchive {
    let mut entries: Vec<(Box<str>, MockContent)> = vec![
        (DIR.into(), MockContent::Dir),
        (
            READER_PATH.into(),
            MockContent::Reader(READER_CONTENTS.as_bytes().into()),
        ),
    ];

    if let Some(p) = file {
        entries.push((FILE_PATH.into(), MockContent::File(p)));
    }

    MockArchive::from_entries(entries)
}

/// No bmap and no customizations: `flash`'s other generics still need concrete
/// types, and these are the shapes the crate's own callers use.
fn no_bmap() -> Option<fn() -> io::Result<Box<str>>> {
    None
}

type NoCustomizations =
    std::iter::Empty<Customization<std::iter::Empty<(Box<str>, ContentType<'static>)>>>;

fn no_customizations() -> NoCustomizations {
    std::iter::empty()
}

#[test]
fn flash_applies_bootfs_through_public_api() {
    let mut mock = MockSd::new();
    let mut img = mock.image_copy();
    img.rewind().unwrap();
    let img_size = mock.size();

    const FILE_CONTENTS: &str = "contents pulled in from a path";
    let mut temp_file = tempfile::NamedTempFile::new().unwrap();
    temp_file.write_all(FILE_CONTENTS.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let archive = archive(Some(temp_file.path().into()));

    bb_flasher_sd::flashing::Flasher::new(
        move || Ok((img, img_size)),
        Some(move || Ok(archive)),
        no_bmap(),
        no_customizations(),
        None,
        None,
    )
    .flash(Destination::File(mock.path().into()), false)
    .expect("flash with a bootfs archive should succeed");

    let fs = mock.open_boot();
    let root = fs.root_dir();

    root.open_dir(DIR)
        .expect("bootfs directory should exist in boot partition");

    let mut contents = String::new();
    root.open_file(READER_PATH)
        .expect("bootfs reader entry should exist in boot partition")
        .read_to_string(&mut contents)
        .unwrap();
    assert_eq!(contents, READER_CONTENTS);

    contents.clear();
    root.open_file(FILE_PATH)
        .expect("bootfs file entry should exist in boot partition")
        .read_to_string(&mut contents)
        .unwrap();
    assert_eq!(contents, FILE_CONTENTS);
}

/// The bootfs archive is applied after the image write and before
/// customizations. Appending to a file the archive created proves both halves
/// of that ordering: the image write did not clobber the archive's output, and
/// the customization ran on top of it.
#[test]
fn flash_applies_bootfs_before_customizations() {
    const APPENDED: &str = " and then by the customization";

    let mut mock = MockSd::new();
    let mut img = mock.image_copy();
    img.rewind().unwrap();
    let img_size = mock.size();

    let archive = archive(None);

    // `ContentType` is not Send, so the content iterator constructs it lazily
    // from Send inputs via `map` to satisfy `flash`'s `+ Send` bound.
    let content = vec![(
        READER_PATH.into(),
        APPENDED.as_bytes().to_vec().into_boxed_slice(),
    )]
    .into_iter()
    .map(|(name, data): (Box<str>, Box<[u8]>)| (name, ContentType::DataAppend(data)));

    bb_flasher_sd::flashing::Flasher::new(
        move || Ok((img, img_size)),
        Some(move || Ok(archive)),
        no_bmap(),
        std::iter::once(Customization {
            partition: ParitionType::Boot,
            content,
        }),
        None,
        None,
    )
    .flash(Destination::File(mock.path().into()), false)
    .expect("flash with bootfs and customization should succeed");

    let fs = mock.open_boot();
    let mut contents = String::new();
    fs.root_dir()
        .open_file(READER_PATH)
        .expect("bootfs reader entry should exist in boot partition")
        .read_to_string(&mut contents)
        .unwrap();

    assert_eq!(
        contents,
        format!("{}{APPENDED}", READER_CONTENTS),
        "customization should append to the file the bootfs archive wrote"
    );
}

#[test]
fn flash_propagates_bootfs_resolver_error() {
    let mock = MockSd::new();
    let img = mock.image_copy();
    let img_size = mock.size();

    let bootfs = || -> io::Result<MockArchive> {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "bootfs archive missing",
        ))
    };

    let result = bb_flasher_sd::flashing::Flasher::new(
        move || Ok((img, img_size)),
        Some(bootfs),
        no_bmap(),
        no_customizations(),
        None,
        None,
    )
    .flash(Destination::File(mock.path().into()), false);

    assert!(
        matches!(
            result,
            Err(bb_flasher_sd::Error::IoError { ref source }) if source.kind() == io::ErrorKind::NotFound
        ),
        "a failing bootfs resolver should surface as an IO error, got {result:?}"
    );
}
