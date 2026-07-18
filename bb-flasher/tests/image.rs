#![cfg(feature = "sd")]

//! Integration tests for the public `LocalImage` type and `img::OsArchive`.
//! `OsImage` compression detection is already covered by inline tests in
//! src/img/test.rs, but `LocalImage` and `OsArchive` had no coverage.

use std::io::Read;

use bb_flasher::LocalImage;
use bb_flasher::img::OsArchive;
use bb_flasher_sd::ContentType;

fn write_temp(dir: &std::path::Path, name: &str, data: &[u8]) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, data).unwrap();
    path
}

#[test]
fn local_image_accessors_and_display() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp(dir.path(), "myimage.img", b"raw image bytes");

    let img = LocalImage::new(path.clone().into_boxed_path());
    assert_eq!(img.path(), path.as_path());
    assert_eq!(img.file_name(), "myimage.img");
    // Display renders the file name.
    assert_eq!(img.to_string(), "myimage.img");
}

#[test]
fn local_image_into_image_fn_reads_uncompressed_contents() {
    let dir = tempfile::tempdir().unwrap();
    let data = b"raw uncompressed image payload";
    let path = write_temp(dir.path(), "os.img", data);

    let resolver = LocalImage::new(path.into_boxed_path()).into_image_fn();
    let (mut img, size) = resolver().unwrap();

    assert_eq!(size, data.len() as u64);
    let mut out = Vec::new();
    img.read_to_end(&mut out).unwrap();
    assert_eq!(out, data);
}

/// Build an uncompressed tar containing one directory and one file.
fn build_tar() -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut bytes);

        let mut dir_header = tar::Header::new_gnu();
        dir_header.set_entry_type(tar::EntryType::Directory);
        dir_header.set_size(0);
        dir_header.set_mode(0o755);
        dir_header.set_cksum();
        builder.append_data(&mut dir_header, "config", std::io::empty()).unwrap();

        let contents = b"tar file contents";
        let mut file_header = tar::Header::new_gnu();
        file_header.set_entry_type(tar::EntryType::Regular);
        file_header.set_size(contents.len() as u64);
        file_header.set_mode(0o644);
        file_header.set_cksum();
        builder
            .append_data(&mut file_header, "config/hello.txt", contents.as_slice())
            .unwrap();

        builder.finish().unwrap();
    }
    bytes
}

/// Iterate an OsArchive, asserting the directory and file entries are surfaced.
/// Entry readers must be consumed in order (tar is sequential), so this reads
/// inline rather than collecting first.
fn assert_archive_entries(archive: &mut OsArchive) {
    let mut saw_dir = false;
    let mut saw_file = false;

    for (name, content) in archive.into_iter() {
        match content {
            ContentType::Dir => {
                assert_eq!(&*name, "config");
                saw_dir = true;
            }
            ContentType::Reader(mut reader) => {
                assert_eq!(&*name, "config/hello.txt");
                let mut s = String::new();
                reader.read_to_string(&mut s).unwrap();
                assert_eq!(s, "tar file contents");
                saw_file = true;
            }
            _ => panic!("unexpected content type for {name}"),
        }
    }

    assert!(saw_dir, "directory entry missing");
    assert!(saw_file, "file entry missing");
}

#[test]
fn os_archive_iterates_plain_tar() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp(dir.path(), "archive.tar", &build_tar());

    let mut archive = OsArchive::from_path(&path, None).unwrap();
    assert_archive_entries(&mut archive);
}

#[test]
fn os_archive_iterates_tar_xz() {
    let dir = tempfile::tempdir().unwrap();
    let compressed = liblzma::encode_all(build_tar().as_slice(), 6).unwrap();
    let path = write_temp(dir.path(), "archive.tar.xz", &compressed);

    let mut archive = OsArchive::from_path(&path, None).unwrap();
    assert_archive_entries(&mut archive);
}
