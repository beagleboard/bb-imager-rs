#![cfg(feature = "sd")]

//! Tests of [`bb_flasher::sd::Flasher::flash`]'s `bootfs` argument.
//!
//! `tests/sd.rs` covers the plain image write. These tests cover the archive
//! that is unpacked into the BOOT partition after the image has been written
//! and before the customizations are applied, going through the same
//! `LocalImage`-based resolvers the front-ends use, so the tar/tar.xz decoding
//! in `img::OsArchive` is exercised along with the flashing itself.

use std::io::{Read, Seek, Write};
use std::sync::mpsc;

use bb_flasher::sd::{FlashingSdLinuxConfig, NONE_BOOTFS};
use bb_flasher::{DownloadFlashingStatus, LocalImage};
use bb_flasher_sd::mock_sd::MockSd;
use tempfile::NamedTempFile;

const DIR: &str = "bootfs_dir";
const FILE: &str = "bootfs_dir/cmdline.txt";
const FILE_CONTENTS: &str = "console=ttyS0,115200n8";
const SYSCONF: &str = "sysconf.txt";
const SYSCONF_CONTENTS: &str = "# shipped by the bootfs archive\n";

/// `flash`'s bmap argument is generic, so a bare `None` gives the compiler
/// nothing to infer from. None of these tests use a bmap.
fn no_bmap() -> Option<fn() -> std::io::Result<Box<str>>> {
    None
}

/// The BOOT entries flashed by these tests: a directory, a file inside it, and
/// a `sysconf.txt` that the customization test appends to.
///
/// Directory entries carry the trailing slash GNU tar writes, since that is
/// what real image tarballs contain.
fn tar_bytes() -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());

    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Directory);
    header.set_mode(0o755);
    header.set_size(0);
    builder
        .append_data(&mut header, format!("{DIR}/"), std::io::empty())
        .unwrap();

    for (path, contents) in [(FILE, FILE_CONTENTS), (SYSCONF, SYSCONF_CONTENTS)] {
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Regular);
        header.set_mode(0o644);
        header.set_size(contents.len() as u64);
        builder
            .append_data(&mut header, path, contents.as_bytes())
            .unwrap();
    }

    builder.into_inner().unwrap()
}

fn temp_file(data: &[u8]) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(data).unwrap();
    f.flush().unwrap();
    f
}

/// A tar of [`tar_bytes`], ready to hand to [`LocalImage::into_archive_fn`].
fn bootfs_tar() -> NamedTempFile {
    temp_file(&tar_bytes())
}

/// The same archive, xz compressed, to cover `OsArchive`'s tar.xz branch.
fn bootfs_tar_xz() -> NamedTempFile {
    temp_file(&liblzma::encode_all(tar_bytes().as_slice(), 1).unwrap())
}

fn archive_fn(f: &NamedTempFile) -> impl FnOnce() -> std::io::Result<bb_flasher::img::OsArchive> {
    LocalImage::new(f.path().into()).into_archive_fn(None)
}

/// A freshly created [`MockSd`] is a valid 128 MiB MBR + FAT32 card, so its own
/// bytes make a usable OS image: flashing them onto another `MockSd` leaves a
/// destination whose BOOT partition `open_boot` can inspect.
fn image_fn(src: &MockSd) -> impl FnOnce() -> std::io::Result<(bb_flasher::img::OsImage, u64)> {
    LocalImage::new(src.path().into()).into_image_fn()
}

/// Each `open_boot` starts reading at the card's current offset, so the handle
/// has to be rewound before every inspection.
fn read_boot_file(sd: &mut MockSd, path: &str) -> String {
    sd.rewind().unwrap();
    let fs = sd.open_boot();
    let mut out = String::new();
    fs.root_dir()
        .open_file(path)
        .unwrap_or_else(|e| panic!("{path} should exist in BOOT partition: {e}"))
        .read_to_string(&mut out)
        .unwrap();
    out
}

#[test]
fn flash_writes_bootfs_entries_to_boot_partition() {
    let src = MockSd::new();
    let mut dst = MockSd::new();
    let archive = bootfs_tar();

    bb_flasher::sd::Flasher::with_file_dest(
        image_fn(&src),
        Some(archive_fn(&archive)),
        no_bmap(),
        dst.path().to_path_buf(),
        FlashingSdLinuxConfig::none(),
    )
    .flash(None, None)
    .expect("flashing with a bootfs archive should succeed");

    dst.rewind().unwrap();
    dst.open_boot()
        .root_dir()
        .open_dir(DIR)
        .expect("bootfs directory should exist in BOOT partition");

    assert_eq!(read_boot_file(&mut dst, FILE), FILE_CONTENTS);
    assert_eq!(read_boot_file(&mut dst, SYSCONF), SYSCONF_CONTENTS);
}

#[test]
fn flash_accepts_xz_compressed_bootfs_archive() {
    let src = MockSd::new();
    let mut dst = MockSd::new();
    let archive = bootfs_tar_xz();

    bb_flasher::sd::Flasher::with_file_dest(
        image_fn(&src),
        Some(archive_fn(&archive)),
        no_bmap(),
        dst.path().to_path_buf(),
        FlashingSdLinuxConfig::none(),
    )
    .flash(None, None)
    .expect("flashing with an xz compressed bootfs archive should succeed");

    assert_eq!(read_boot_file(&mut dst, FILE), FILE_CONTENTS);
}

/// The archive is unpacked after the image write and before customizations.
/// Letting the customization append to a file the archive created proves both
/// halves of that ordering: the image write did not clobber the archive's
/// output, and the customization ran on top of it.
#[test]
fn flash_applies_bootfs_before_customization() {
    let src = MockSd::new();
    let mut dst = MockSd::new();
    let archive = bootfs_tar();

    bb_flasher::sd::Flasher::with_file_dest(
        image_fn(&src),
        Some(archive_fn(&archive)),
        no_bmap(),
        dst.path().to_path_buf(),
        FlashingSdLinuxConfig::sysconfig(Some("beagle".into()), None, None, None, None, None, None),
    )
    .flash(None, None)
    .expect("flashing with a bootfs archive and customization should succeed");

    assert_eq!(
        read_boot_file(&mut dst, SYSCONF),
        format!("{SYSCONF_CONTENTS}hostname=beagle\n"),
        "customization should append to the file the bootfs archive wrote"
    );
}

#[test]
fn flash_reports_progress_with_bootfs() {
    let src = MockSd::new();
    let mut dst = MockSd::new();
    let archive = bootfs_tar();

    let (tx, rx) = mpsc::sync_channel(32);

    let handle = std::thread::spawn(move || {
        bb_flasher::sd::Flasher::with_file_dest(
            image_fn(&src),
            Some(archive_fn(&archive)),
            no_bmap(),
            dst.path().to_path_buf(),
            FlashingSdLinuxConfig::none(),
        )
        .flash(Some(tx), None)
        .expect("flashing with a bootfs archive should succeed");

        assert_eq!(read_boot_file(&mut dst, FILE), FILE_CONTENTS);
    });

    let progress: Vec<DownloadFlashingStatus> = rx.into_iter().collect();

    assert_eq!(
        progress.first(),
        Some(&DownloadFlashingStatus::Preparing),
        "flashing should start by reporting Preparing"
    );
    assert!(
        progress
            .iter()
            .any(|x| matches!(x, DownloadFlashingStatus::DownloadingProgress(_))),
        "file destinations should report download progress: {progress:?}"
    );

    handle.join().unwrap();
}

#[test]
fn flash_propagates_bootfs_resolver_error() {
    let src = MockSd::new();
    let dst = MockSd::new();

    let bootfs = || -> std::io::Result<bb_flasher::img::OsArchive> {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "bootfs archive missing",
        ))
    };

    let err = bb_flasher::sd::Flasher::with_file_dest(
        image_fn(&src),
        Some(bootfs),
        no_bmap(),
        dst.path().to_path_buf(),
        FlashingSdLinuxConfig::none(),
    )
    .flash(None, None)
    .expect_err("a failing bootfs resolver should fail the flash");

    assert!(
        err.downcast_ref::<bb_flasher_sd::Error>().is_some_and(|e| {
            matches!(e, bb_flasher_sd::Error::IoError { source }
                if source.kind() == std::io::ErrorKind::NotFound)
        }),
        "expected the resolver's io error, got: {err:?}"
    );
}

#[test]
fn flash_without_bootfs_leaves_boot_partition_as_flashed() {
    let src = MockSd::new();
    let mut dst = MockSd::new();

    bb_flasher::sd::Flasher::with_file_dest(
        image_fn(&src),
        NONE_BOOTFS,
        no_bmap(),
        dst.path().to_path_buf(),
        FlashingSdLinuxConfig::none(),
    )
    .flash(None, None)
    .expect("flashing without a bootfs archive should succeed");

    dst.rewind().unwrap();
    let fs = dst.open_boot();
    assert!(
        fs.root_dir().open_dir(DIR).is_err(),
        "no bootfs archive should leave the image's BOOT partition alone"
    );
}
