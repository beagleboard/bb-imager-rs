use super::*;

#[test]
fn normalize_file_dest_strips_known_suffixes() {
    assert_eq!(normalize_file_dest("os.zip"), "os");
    assert_eq!(normalize_file_dest("os.img.xz"), "os.img");
    assert_eq!(normalize_file_dest("os.img.gz"), "os.img");
    assert_eq!(normalize_file_dest("plain.txt"), "plain.txt");
}

#[test]
fn flasher_supported_matches_enabled_features() {
    // const fn whose arms are feature-gated; compare against cfg! so the
    // assertion holds under any feature set the suite is compiled with.
    assert_eq!(
        flasher_supported(config::Flasher::SdCard),
        cfg!(feature = "sd")
    );
    assert_eq!(
        flasher_supported(config::Flasher::SdCardBootfs),
        cfg!(feature = "sd")
    );
    assert_eq!(
        flasher_supported(config::Flasher::SdCardNoBootloader),
        cfg!(feature = "sd")
    );
    assert_eq!(
        flasher_supported(config::Flasher::BeagleConnectFreedom),
        cfg!(feature = "bcf_cc1352p7")
    );
    assert_eq!(
        flasher_supported(config::Flasher::Msp430Usb),
        cfg!(feature = "bcf_msp430")
    );
    assert_eq!(
        flasher_supported(config::Flasher::Mspm0),
        cfg!(any(feature = "zepto_uart", feature = "zepto_i2c"))
    );
}

/// A remote SD image with the given init format.
///
/// Remote specifically: a local image reports `InitFormat::None` and takes
/// the `is_local` branch instead, so it cannot exercise format detection.
fn remote_sd_image(flasher: config::Flasher, init_format: config::InitFormat) -> BoardImage {
    let cache = tempfile::tempdir().unwrap();
    let downloader = bb_downloader::Downloader::new(cache.path()).unwrap();

    let os_image = crate::db::OsImage {
        id: 1,
        name: "test-image".into(),
        url: Box::new(url::Url::parse("https://example.com/os.img.xz").unwrap()),
        image_download_size: 0,
        image_download_sha256: [0u8; 32],
        extract_size: 0,
        init_format,
        bmap: None,
        info_text: None,
    };

    BoardImage::Image {
        flasher,
        init_format,
        img: RemoteImage::new(&os_image, downloader, flasher).into(),
        #[cfg(feature = "sd")]
        bmap: None,
        info_text: None,
    }
}

/// Both SD flashers are the same as far as customization goes; only the
/// bootfs archive differs.
const SD_FLASHERS: [config::Flasher; 2] =
    [config::Flasher::SdCard, config::Flasher::SdCardNoBootloader];

/// Returning `Some` for a customizable image skips the Customize page and
/// writes an empty config, so the user silently loses hostname, user, wifi
/// and SSH key.
///
/// The two formats are checked by separate guards, so it is possible to fix
/// or break one without touching the other.
#[test]
fn customizable_init_formats_reach_the_customization_page() {
    for flasher in SD_FLASHERS {
        for format in [config::InitFormat::Sysconf, config::InitFormat::CloudInit] {
            let img = remote_sd_image(flasher, format);
            assert!(
                no_customization(flasher, &img).is_none(),
                "{format:?} images must be customizable on {flasher:?}"
            );
        }
    }
}

/// The counterpart: formats we cannot write skip the page rather than
/// showing one that would do nothing.
#[test]
fn unwritable_init_formats_skip_the_customization_page() {
    for flasher in SD_FLASHERS {
        for format in [config::InitFormat::None, config::InitFormat::Armbian] {
            let img = remote_sd_image(flasher, format);
            assert!(
                matches!(
                    no_customization(flasher, &img),
                    Some(FlashingCustomization::NoneSd)
                ),
                "{format:?} images have no customization we can apply on {flasher:?}"
            );
        }
    }
}

/// Local images carry no detected format, so they get the picker.
#[test]
fn local_images_reach_the_customization_page() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let img = BoardImage::local(file.path().to_path_buf(), config::Flasher::SdCard);

    assert!(no_customization(config::Flasher::SdCard, &img).is_none());
}

#[test]
fn no_customization_covers_non_configurable_flashers() {
    let img = BoardImage::SdFormat;
    assert!(matches!(
        no_customization(config::Flasher::SdCard, &img),
        Some(FlashingCustomization::NoneSd)
    ));
    assert!(matches!(
        no_customization(config::Flasher::SdCardBootfs, &img),
        Some(FlashingCustomization::NoneSd)
    ));
    assert!(matches!(
        no_customization(config::Flasher::SdCardNoBootloader, &img),
        Some(FlashingCustomization::NoneSd)
    ));
    assert!(matches!(
        no_customization(config::Flasher::Msp430Usb, &img),
        Some(FlashingCustomization::Msp430)
    ));
    // BCF and MSPM0 have no customization of their own, so they skip the page.
    assert!(matches!(
        no_customization(config::Flasher::BeagleConnectFreedom, &img),
        Some(FlashingCustomization::Bcf)
    ));
    assert!(matches!(
        no_customization(config::Flasher::Mspm0, &img),
        Some(FlashingCustomization::Zepto)
    ));
}

#[test]
fn board_image_format_accessors() {
    let img = BoardImage::SdFormat;
    assert_eq!(img.flasher(), config::Flasher::SdCard);
    assert_eq!(img.init_format(), config::InitFormat::None);
    assert_eq!(img.info_text(), None);
    // Nothing to write out, so the destination page offers no "Save To File".
    assert_eq!(img.file_name(), None);
    assert_eq!(img.to_string(), "Format SD Card");
}

#[test]
fn board_image_local_reads_file_metadata() {
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), b"0123456789").unwrap();

    let img = BoardImage::local(
        file.path().to_path_buf(),
        config::Flasher::BeagleConnectFreedom,
    );
    assert_eq!(img.flasher(), config::Flasher::BeagleConnectFreedom);
    assert_eq!(img.init_format(), config::InitFormat::None);
    assert!(img.file_name().is_some_and(|n| !n.is_empty()));
}

#[test]
fn destination_local_file_behaviour() {
    let dst = Destination::LocalFile(PathBuf::from("/tmp/os.img"));
    assert!(dst.is_download_action());
    assert_eq!(dst.size(), None);
    assert_eq!(dst.to_string(), "Save To File");
}

#[test]
fn system_keymap_is_never_empty() {
    // Falls back to "us" when the locale cannot be resolved.
    assert!(!system_keymap().is_empty());
}

/// `SdCardNoBootloader` is an SD flasher in every respect except the bootfs
/// archive, so it must offer the same destinations and file types. These
/// matches end in a catch-all, so a missed variant panics at runtime rather
/// than failing to compile.
#[cfg(feature = "sd")]
#[test]
fn no_bootloader_flasher_behaves_like_sd_card() {
    assert_eq!(
        file_filter(config::Flasher::SdCardNoBootloader),
        file_filter(config::Flasher::SdCard)
    );
    // Enumeration hits the same catch-all; the call itself is the assertion.
    let _ = destinations(config::Flasher::SdCardNoBootloader, false, "".into());
}

/// The board's bootfs archive, as `start_flashing` builds it.
#[cfg(feature = "sd")]
fn board_bootfs() -> RemoteItem {
    let cache = tempfile::tempdir().unwrap();

    RemoteItem::new(
        Box::new(url::Url::parse("https://example.com/bootfs.tar.xz").unwrap()),
        [7u8; 32],
        4096,
        bb_downloader::Downloader::new(cache.path()).unwrap(),
    )
}

/// A local image flashed to a file, so the flash needs no SD card and no
/// elevated permissions.
#[cfg(feature = "sd")]
async fn flash_local_image(
    flasher: config::Flasher,
    bootfs: Option<RemoteItem>,
) -> (anyhow::Result<()>, tempfile::NamedTempFile, Vec<u8>) {
    use std::io::Write;

    let data: Vec<u8> = (0..4096u32).map(|x| (x % 251) as u8).collect();
    let mut src = tempfile::NamedTempFile::new().unwrap();
    src.write_all(&data).unwrap();
    src.flush().unwrap();

    let dst = tempfile::NamedTempFile::new().unwrap();
    let res = flash(
        BoardImage::local(src.path().to_path_buf(), flasher),
        FlashingCustomization::NoneSd,
        Destination::LocalFile(dst.path().to_path_buf()),
        bootfs,
        mpsc::sync_channel(8).0,
        bb_helper::cancel::CancellationToken::default(),
    )
    .await;

    (res, dst, data)
}

/// The board's archive is handed to every SD flash, so only the flasher type
/// decides who writes it. The archive here points at a host that does not
/// resolve, so applying it to a plain SD image would fail the flash.
#[cfg(feature = "sd")]
#[tokio::test]
async fn plain_sd_image_ignores_the_boards_bootfs() {
    let (res, dst, data) = flash_local_image(config::Flasher::SdCard, Some(board_bootfs())).await;

    res.expect("a plain SD image must not pick up the board's bootfs");
    assert_eq!(std::fs::read(dst.path()).unwrap(), data);
}
