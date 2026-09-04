#![cfg(feature = "sd")]

use std::io::{Read, Write};
use std::sync::mpsc;

use bb_flasher::{
    BBFlasherTarget, DownloadFlashingStatus, img::OsImage, sd::FlashingSdLinuxConfig,
};
use bb_flasher_sd::mock_sd::MockSd;
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
        bb_flasher::sd::NONE_BOOTFS,
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
            bb_flasher::sd::NONE_BOOTFS,
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
        bb_flasher::sd::NONE_BOOTFS,
        None::<Box<dyn FnOnce() -> std::io::Result<Box<str>> + Send>>,
        sd.path().to_path_buf(),
        FlashingSdLinuxConfig::none(),
    )
    .flash(None, Some(cancel));

    assert!(res.is_err());
}

const WIFI_PSK: &str = "[Security]\nPassphrase=hunter2\n\n[Settings]\nAutoConnect=true";

/// Flash `mock` with Wi-Fi customization, using a copy of its own bytes as the
/// OS image (flashing truncates the destination).
fn flash_with_wifi(mock: &MockSd) {
    let image = mock.image_copy();
    let img_path = image.path().to_path_buf();

    bb_flasher::sd::Flasher::with_file_dest(
        move || {
            let size = std::fs::metadata(&img_path)?.len();
            Ok((OsImage::from_path(&img_path)?, size))
        },
        bb_flasher::sd::NONE_BOOTFS,
        None::<Box<dyn FnOnce() -> std::io::Result<Box<str>> + Send>>,
        mock.path().to_path_buf(),
        FlashingSdLinuxConfig::sysconfig(
            None,
            None,
            None,
            None,
            Some(("mynet".into(), "hunter2".into())),
            None,
            None,
        ),
    )
    .flash(None, None)
    .unwrap();
}

/// Wi-Fi customization splits across two boot-partition files: `sysconf.txt`
/// gains an `iwd_psk_file` key pointing at a per-SSID PSK file, and the PSK file
/// itself is written under `services/`.
#[test]
fn flash_wifi_writes_psk_into_boot_partition() {
    let mut mock = MockSd::new();

    flash_with_wifi(&mock);

    assert_eq!(
        mock.boot_file("sysconf.txt").unwrap(),
        "iwd_psk_file=mynet.psk\n"
    );
    assert_eq!(mock.boot_file("services/mynet.psk").unwrap(), WIFI_PSK);
}

/// The counterpart to [`flash_wifi_writes_psk_into_boot_partition`]: real images
/// ship a `services/` directory, so the `ContentType::Dir` entry customization
/// emits for it has to be a no-op rather than an error, and must leave what the
/// image already put there alone.
#[test]
fn flash_wifi_reuses_existing_services_dir() {
    let mut mock = MockSd::new();

    // Seed the image with `services/` and an unrelated file inside it.
    {
        let fs = mock.open_boot();
        fs.root_dir().create_dir("services").unwrap();
        fs.root_dir()
            .create_file("services/other.psk")
            .unwrap()
            .write_all(b"pre-existing")
            .unwrap();
        fs.unmount().unwrap();
    }

    flash_with_wifi(&mock);

    assert_eq!(mock.boot_file("services/mynet.psk").unwrap(), WIFI_PSK);
    assert_eq!(
        mock.boot_file("services/other.psk").unwrap(),
        "pre-existing",
        "customization must not clobber a `services/` directory the image shipped"
    );
}

/// The GPT counterpart to [`flash_wifi_writes_psk_into_boot_partition`]: the
/// same sysconf customization has to land when the boot partition is found
/// through the GPT table instead of the MBR one.
#[test]
fn flash_wifi_writes_psk_into_gpt_boot_partition() {
    let mut mock = MockSd::new_gpt();

    flash_with_wifi(&mock);

    assert_eq!(
        mock.boot_file("sysconf.txt").unwrap(),
        "iwd_psk_file=mynet.psk\n"
    );
    assert_eq!(mock.boot_file("services/mynet.psk").unwrap(), WIFI_PSK);
}

#[test]
fn destinations() {
    let temp = bb_flasher::sd::Target::destinations(false);
    assert!(!temp.count() > 0);
}
