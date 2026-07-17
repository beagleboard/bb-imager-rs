#![cfg(feature = "sd")]

use std::io::{Read, Write};
use std::sync::mpsc;

use bb_flasher::{
    BBFlasherTarget, DownloadFlashingStatus, img::OsImage, sd::FlashingSdLinuxConfig,
};
use bb_helper::cancel::CancellationToken;
use tempfile::NamedTempFile;

const MOCK_IMG_LEN: usize = 1024 * 10;

fn test_file() -> impl Iterator<Item = u8> {
    (0..).map(|x| x % 255).map(|x| u8::try_from(x).unwrap())
}

fn mock_img_data() -> Vec<u8> {
    test_file().take(MOCK_IMG_LEN).collect()
}

fn mock_img() -> OsImage {
    let data: Vec<u8> = mock_img_data();
    let mut f = tempfile::NamedTempFile::new().unwrap();

    std::io::copy(&mut data.as_slice(), &mut f).unwrap();
    f.flush().unwrap();

    OsImage::from_path(f.path()).unwrap()
}

#[test]
fn flash_no_progress() {
    let mut sd = NamedTempFile::new().unwrap();

    bb_flasher::sd::Flasher::with_file_dest(
        || Ok((mock_img(), MOCK_IMG_LEN as u64)),
        None::<Box<dyn FnOnce() -> std::io::Result<Box<str>> + Send>>,
        sd.path().to_path_buf(),
        FlashingSdLinuxConfig::none(),
    )
    .flash(None, None)
    .unwrap();

    let mock_img_data = mock_img_data();
    let mut data = Vec::new();

    sd.read_to_end(&mut data).unwrap();

    assert_eq!(data, mock_img_data);
}

#[test]
fn flash_progress() {
    let mut sd = NamedTempFile::new().unwrap();

    let (tx, rx) = mpsc::sync_channel(32);

    let handle = std::thread::spawn(move || {
        bb_flasher::sd::Flasher::with_file_dest(
            || Ok((mock_img(), MOCK_IMG_LEN as u64)),
            None::<Box<dyn FnOnce() -> std::io::Result<Box<str>> + Send>>,
            sd.path().to_path_buf(),
            FlashingSdLinuxConfig::none(),
        )
        .flash(Some(tx), None)
        .unwrap();

        let mock_img_data = mock_img_data();
        let mut data = Vec::new();

        sd.read_to_end(&mut data).unwrap();

        assert_eq!(data, mock_img_data);
    });

    // 8. Verify progress track completeness
    let progress_updates: Vec<DownloadFlashingStatus> = rx.into_iter().collect();
    assert!(!progress_updates.is_empty());
    assert_eq!(
        *progress_updates.first().unwrap(),
        DownloadFlashingStatus::Preparing
    );

    handle.join().unwrap();
}

#[test]
fn flash_cancel() {
    let sd = NamedTempFile::new().unwrap();
    let cancel = CancellationToken::default();

    drop(cancel.drop_guard());

    let res = bb_flasher::sd::Flasher::with_file_dest(
        || Ok((mock_img(), MOCK_IMG_LEN as u64)),
        None::<Box<dyn FnOnce() -> std::io::Result<Box<str>> + Send>>,
        sd.path().to_path_buf(),
        FlashingSdLinuxConfig::none(),
    )
    .flash(None, Some(cancel));

    assert!(res.is_err());
}

#[test]
fn destinations() {
    let temp = bb_flasher::sd::Target::destinations(false);
    assert!(!temp.is_empty());
}
