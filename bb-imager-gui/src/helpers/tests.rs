use super::*;
use crate::persistance::{
    GuiConfiguration, SdCustomizationUser, SdCustomizationWifi, SdSysconfCustomization,
};

#[test]
fn pretty_bytes_scales_units() {
    assert_eq!(pretty_bytes(0), "0 B");
    assert_eq!(pretty_bytes(512), "512 B");
    assert_eq!(pretty_bytes(1024), "1.00 KiB");
    assert_eq!(pretty_bytes(1536), "1.50 KiB");
    assert_eq!(pretty_bytes(1024 * 1024), "1.00 MiB");
    assert_eq!(pretty_bytes(1024 * 1024 * 1024), "1.00 GiB");
}

#[test]
fn pretty_duration_formats_minutes_and_seconds() {
    assert_eq!(pretty_duration(Duration::from_secs(0)), "0s");
    assert_eq!(pretty_duration(Duration::from_secs(45)), "45s");
    assert_eq!(pretty_duration(Duration::from_secs(60)), "1:00");
    assert_eq!(pretty_duration(Duration::from_secs(125)), "2:05");
}

#[test]
fn normalize_file_dest_strips_known_suffixes() {
    assert_eq!(normalize_file_dest("os.zip"), "os");
    assert_eq!(normalize_file_dest("os.img.xz"), "os.img");
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

#[test]
fn sd_modifications_common_lists_configured_fields() {
    assert!(sd_modifications_common(&SdSysconfCustomization::default()).is_empty());

    let full = SdSysconfCustomization::default()
        .update_hostname(Some("h".into()))
        .update_timezone(Some("UTC".parse().unwrap()))
        .update_keymap(Some("us".into()))
        .update_ssh(Some("k".into()))
        .update_user(Some(SdCustomizationUser::new("u".into(), "p".into())))
        .update_wifi(Some(SdCustomizationWifi::default()));
    let mods = sd_modifications_common(&full);
    assert_eq!(mods.len(), 6);
    assert!(mods.contains(&"User account configured"));
    assert!(mods.contains(&"Wifi configured"));
    assert!(mods.contains(&"SSH Key configured"));
}

#[test]
fn modifications_reports_only_what_gets_written() {
    // Nothing is applied for these, so the review page shows no list.
    for c in [
        FlashingCustomization::NoneSd,
        FlashingCustomization::Msp430,
        FlashingCustomization::Bcf,
        FlashingCustomization::Zepto,
    ] {
        assert!(c.modifications().is_empty());
    }

    let hostname = SdSysconfCustomization::default().update_hostname(Some("h".into()));
    assert_eq!(
        FlashingCustomization::LinuxSdCloudInit(hostname.clone()).modifications(),
        ["Hostname configured"].into()
    );

    // USB DHCP is sysconf-only: cloud-init has no such field to write.
    let dhcp = hostname.update_usb_enable_dhcp(Some(true));
    assert_eq!(
        FlashingCustomization::LinuxSdSysconfig(dhcp.clone()).modifications(),
        ["Hostname configured", "USB DHCP enabled"].into()
    );
    assert_eq!(
        FlashingCustomization::LinuxSdCloudInit(dhcp).modifications(),
        ["Hostname configured"].into()
    );
}

/// A remote SD image with the given init format.
///
/// Remote specifically: a local image always reports `InitFormat::None`, so
/// it cannot exercise format detection.
fn remote_sd_image(flasher: config::Flasher, init_format: config::InitFormat) -> BoardImage {
    let cache = tempfile::tempdir().unwrap();
    let downloader = bb_downloader::Downloader::new(cache.path()).unwrap();

    BoardImage::remote(
        crate::db::OsImage {
            id: 1,
            name: "test-image".into(),
            description: "test".to_string(),
            icon: std::sync::Arc::new(url::Url::parse("https://example.com/icon.png").unwrap()),
            url: Box::new(url::Url::parse("https://example.com/os.img.xz").unwrap()),
            image_download_size: 0,
            image_download_sha256: [0u8; 32],
            extract_size: 0,
            release_date: chrono::NaiveDate::from_ymd_opt(2024, 5, 10).unwrap(),
            init_format,
            bmap: None,
            info_text: None,
            support: None,
        },
        flasher,
        downloader,
    )
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

#[test]
fn no_customization_covers_non_configurable_flashers() {
    let img = BoardImage::format();
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
fn flashing_customization_new_selects_variant_by_flasher() {
    // A format image has init_format None, so SD falls through to NoneSd.
    let img = BoardImage::format();
    let cfg = GuiConfiguration::default();

    assert!(matches!(
        FlashingCustomization::new(config::Flasher::SdCard, &img, &cfg),
        FlashingCustomization::NoneSd
    ));
    assert!(matches!(
        FlashingCustomization::new(config::Flasher::SdCardNoBootloader, &img, &cfg),
        FlashingCustomization::NoneSd
    ));
    assert!(matches!(
        FlashingCustomization::new(config::Flasher::BeagleConnectFreedom, &img, &cfg),
        FlashingCustomization::Bcf
    ));
    assert!(matches!(
        FlashingCustomization::new(config::Flasher::Msp430Usb, &img, &cfg),
        FlashingCustomization::Msp430
    ));
    assert!(matches!(
        FlashingCustomization::new(config::Flasher::Mspm0, &img, &cfg),
        FlashingCustomization::Zepto
    ));
}

#[test]
fn flashing_customization_validate_checks_user() {
    assert!(FlashingCustomization::NoneSd.validate());
    assert!(FlashingCustomization::LinuxSdSysconfig(SdSysconfCustomization::default()).validate());
    let root = SdSysconfCustomization::default()
        .update_user(Some(SdCustomizationUser::new("root".into(), "p".into())));
    assert!(!FlashingCustomization::LinuxSdSysconfig(root.clone()).validate());
    // Cloud-init writes the same user account, so it needs the same check.
    assert!(FlashingCustomization::LinuxSdCloudInit(SdSysconfCustomization::default()).validate());
    assert!(!FlashingCustomization::LinuxSdCloudInit(root).validate());
}

#[test]
fn flashing_customization_reset_restores_defaults() {
    let mut sysconf = FlashingCustomization::LinuxSdSysconfig(
        SdSysconfCustomization::default().update_hostname(Some("h".into())),
    );
    sysconf.reset();
    match sysconf {
        FlashingCustomization::LinuxSdSysconfig(c) => assert!(c.hostname.is_none()),
        _ => panic!("variant should be preserved"),
    }

    // Cloud-init is just as resettable as sysconf.
    let mut cloudinit = FlashingCustomization::LinuxSdCloudInit(
        SdSysconfCustomization::default().update_hostname(Some("h".into())),
    );
    cloudinit.reset();
    match cloudinit {
        FlashingCustomization::LinuxSdCloudInit(c) => assert!(c.hostname.is_none()),
        _ => panic!("variant should be preserved"),
    }

    // Variants without inner state are left untouched.
    let mut none = FlashingCustomization::NoneSd;
    none.reset();
    assert!(matches!(none, FlashingCustomization::NoneSd));

    let mut bcf = FlashingCustomization::Bcf;
    bcf.reset();
    assert!(matches!(bcf, FlashingCustomization::Bcf));
}

#[test]
fn board_image_format_accessors() {
    let img = BoardImage::format();
    assert_eq!(
        img.description(),
        Some("Format a SD Card to FAT32 for reuse.")
    );
    assert_eq!(img.flasher(), config::Flasher::SdCard);
    assert_eq!(img.init_format(), config::InitFormat::None);
    assert_eq!(img.info_text(), None);
    assert_eq!(img.file_name(), None);
    assert_eq!(img.details(), &[("Format", "FAT32".to_string())]);
    assert!(img.supported_init_formats().is_empty());
    assert!(img.support().is_none());
    assert!(matches!(img.icon(), BoardImageIcon::Format));
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
    assert!(matches!(img.icon(), BoardImageIcon::Local));
    assert!(img.description().is_none());
    assert!(img.file_name().is_some_and(|n| !n.is_empty()));

    let details = img.details();
    assert!(details.iter().any(|(k, _)| *k == "Path"));
    assert!(details.iter().any(|(k, v)| *k == "Size" && v == "10"));
    // Local (non-SD) images offer no init-format customization.
    assert!(img.supported_init_formats().is_empty());
}

#[test]
fn board_image_update_init_format_on_image() {
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), b"x").unwrap();
    let mut img = BoardImage::local(file.path().to_path_buf(), config::Flasher::SdCard);
    img.update_init_format(config::InitFormat::Sysconf);
    assert_eq!(img.init_format(), config::InitFormat::Sysconf);
}

#[test]
fn destination_local_file_behaviour() {
    let dst = Destination::LocalFile(PathBuf::from("/tmp/os.img"));
    assert!(dst.is_download_action());
    assert_eq!(dst.size(), None);
    assert_eq!(dst.details(), vec![("Path", "/tmp/os.img".to_string())]);
    assert_eq!(dst.to_string(), "Save To File");
}

#[test]
fn destination_item_save_to_file() {
    let item = DestinationItem::SaveToFile("os.img.xz".to_string());
    let other = Destination::LocalFile(PathBuf::from("/tmp/x"));

    assert_eq!(item.to_string(), "Save To File");
    assert!(!item.is_selected(&other));
    assert!(item.subtitle().is_none());
    match item.msg() {
        BBImagerMessage::SelectFileDest(name) => assert_eq!(name, "os.img"),
        other => panic!("expected SelectFileDest, got {other:?}"),
    }
}

#[test]
fn destination_item_wraps_destination() {
    let dst = Destination::LocalFile(PathBuf::from("/tmp/os.img"));
    let other = Destination::LocalFile(PathBuf::from("/tmp/other.img"));
    let item = DestinationItem::Destination(&dst);

    assert_eq!(item.to_string(), "Save To File");
    assert!(item.is_selected(&dst));
    assert!(!item.is_selected(&other));
    // LocalFile has no size, so no subtitle.
    assert!(item.subtitle().is_none());
}

#[test]
fn os_image_item_constructors_and_predicates() {
    let local = OsImageItem::local(config::Flasher::SdCard);
    assert_eq!(local.id, OsImageId::Local(config::Flasher::SdCard));
    assert!(!local.is_sublist());
    assert_eq!(local.label(), "Select Local Image");

    let format = OsImageItem::format("Format".into());
    assert_eq!(format.id, OsImageId::Format);
    assert!(!format.is_sublist());
    assert_eq!(format.label(), "Format");
}

#[test]
fn os_image_item_from_db_items() {
    let icon = std::sync::Arc::new(Url::parse("https://example.com/icon.png").unwrap());

    let image: OsImageItem = crate::db::OsImageListItem {
        id: 5,
        icon: icon.clone(),
        name: "Debian".to_string(),
    }
    .into();
    assert_eq!(image.id, OsImageId::OsImage(5));
    assert!(!image.is_sublist());
    assert_eq!(image.label(), "Debian");

    let sublist: OsImageItem = crate::db::OsSublistListItem {
        id: 7,
        icon,
        name: "More".to_string(),
        flasher: config::Flasher::SdCard,
    }
    .into();
    assert_eq!(
        sublist.id,
        OsImageId::OsSublist((7, config::Flasher::SdCard))
    );
    assert!(sublist.is_sublist());
}

#[test]
fn default_user_is_never_empty() {
    assert!(!default_user().is_empty());
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

    for format in [
        config::InitFormat::Sysconf,
        config::InitFormat::CloudInit,
        config::InitFormat::None,
    ] {
        let img = remote_sd_image(config::Flasher::SdCardNoBootloader, format);
        assert_eq!(
            img.supported_init_formats(),
            remote_sd_image(config::Flasher::SdCard, format).supported_init_formats(),
            "{format:?} should offer the same init formats on both SD flashers"
        );
    }
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
