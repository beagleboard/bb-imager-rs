//! End-to-end tests for the `flash sd` / `flash sd-boot-update` subcommands.
//!
//! These drive the real entry point (`Opt::try_parse_from` -> `run`) instead of
//! calling the flasher directly, so argv parsing, the argument -> customization
//! mapping and the progress-rendering thread are all exercised together.
//!
//! `--file-destination` makes the destination a plain file, so no SD card (and
//! no elevated privileges) are needed. Where boot-partition contents matter,
//! the image is a [`MockSd`] (MBR + FAT32) flashed back onto its own path,
//! which lets `MockSd::open_boot` inspect the result.

use std::io::{Read, Seek, Write};

use bb_flasher_sd::mock_sd::MockSd;
use bb_imager_cli::cli::Opt;
use clap::Parser;
use tempfile::NamedTempFile;

/// Run the CLI exactly as `main` would, from an argv.
fn run_cli<const N: usize>(args: [&str; N]) {
    let opt = Opt::try_parse_from(args).expect("argv should parse");
    bb_imager_cli::run(opt);
}

/// A deterministic non-image payload, used where only the raw copy matters.
fn pattern_file(len: usize) -> NamedTempFile {
    let data: Vec<u8> = (0..len).map(|x| (x % 251) as u8).collect();
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(&data).unwrap();
    f.flush().unwrap();
    f
}

/// A `MockSd` plus a copy of its initial bytes stored as a separate file, so
/// the copy can be used as the OS image while the `MockSd` itself is the
/// destination (flashing truncates the destination).
struct SdFixture {
    mock: MockSd,
    image: NamedTempFile,
}

impl SdFixture {
    fn new() -> Self {
        Self::with_boot_dirs(&[])
    }

    /// Like [`Self::new`], but the image's boot partition already contains
    /// `dirs`. Customization writes files with `create_file`, which does not
    /// create missing parent directories, so any nested target needs its parent
    /// to exist in the image beforehand.
    fn with_boot_dirs(dirs: &[&str]) -> Self {
        let mut mock = MockSd::new();

        if !dirs.is_empty() {
            let fs = mock.open_boot();
            for dir in dirs {
                fs.root_dir().create_dir(dir).unwrap();
            }
            fs.unmount().unwrap();
        }

        let mut image = NamedTempFile::new().unwrap();
        let mut src = std::fs::File::open(mock.path()).unwrap();
        std::io::copy(&mut src, image.as_file_mut()).unwrap();
        image.flush().unwrap();

        Self { mock, image }
    }

    fn img(&self) -> &str {
        self.image.path().to_str().unwrap()
    }

    fn dst(&self) -> &str {
        self.mock.path().to_str().unwrap()
    }

    /// Read a file from the boot partition of the flashed result.
    fn boot_file(&mut self, name: &str) -> std::io::Result<String> {
        // `open_boot` reads the partition table from the current cursor.
        self.mock.rewind().unwrap();
        let fs = self.mock.open_boot();
        let mut out = String::new();
        fs.root_dir()
            .open_file(name)
            .map_err(std::io::Error::other)?
            .read_to_string(&mut out)?;
        Ok(out)
    }
}

fn read_all(path: &std::path::Path) -> Vec<u8> {
    std::fs::read(path).unwrap()
}

#[test]
fn flash_sd_file_destination_copies_image_verbatim() {
    let img = pattern_file(64 * 1024);
    let dst = NamedTempFile::new().unwrap();

    run_cli([
        "bb-imager-cli",
        "flash",
        "--quiet",
        "sd",
        img.path().to_str().unwrap(),
        dst.path().to_str().unwrap(),
        "--file-destination",
    ]);

    assert_eq!(read_all(dst.path()), read_all(img.path()));
}

/// Same flash without `--quiet`, which routes progress through the
/// `indicatif`/`console` rendering thread in `run`.
#[test]
fn flash_sd_renders_progress_when_not_quiet() {
    let img = pattern_file(64 * 1024);
    let dst = NamedTempFile::new().unwrap();

    run_cli([
        "bb-imager-cli",
        "flash",
        "sd",
        img.path().to_str().unwrap(),
        dst.path().to_str().unwrap(),
        "--file-destination",
    ]);

    assert_eq!(read_all(dst.path()), read_all(img.path()));
}

/// The destination file is created when it does not already exist.
#[test]
fn flash_sd_creates_missing_destination_file() {
    let img = pattern_file(4 * 1024);
    let dir = tempfile::tempdir().unwrap();
    let dst = dir.path().join("out.img");

    run_cli([
        "bb-imager-cli",
        "flash",
        "--quiet",
        "sd",
        img.path().to_str().unwrap(),
        dst.to_str().unwrap(),
        "--file-destination",
    ]);

    assert_eq!(read_all(&dst), read_all(img.path()));
}

/// With no customization flags the CLI passes `FlashingSdLinuxConfig::none()`,
/// so the boot partition must come out exactly as the image had it.
#[test]
fn flash_sd_without_customization_flags_writes_no_config() {
    let mut fixture = SdFixture::new();

    run_cli([
        "bb-imager-cli",
        "flash",
        "--quiet",
        "sd",
        fixture.img(),
        fixture.dst(),
        "--file-destination",
    ]);

    assert!(
        fixture.boot_file("sysconf.txt").is_err(),
        "sysconf.txt should not be created without customization flags"
    );
}

/// Every sysconfig-backed flag maps to a `key=value` line, in the order
/// `FlashingSdLinuxConfig::sysconfig` writes them.
#[test]
fn flash_sd_sysconfig_writes_every_supplied_key() {
    let mut fixture = SdFixture::new();

    run_cli([
        "bb-imager-cli",
        "flash",
        "--quiet",
        "sd",
        fixture.img(),
        fixture.dst(),
        "--file-destination",
        "--sysconfig",
        "--hostname",
        "beagle",
        "--timezone",
        "Asia/Kolkata",
        "--keymap",
        "us",
        "--user-name",
        "bob",
        "--user-password",
        "hunter2",
        "--ssh-key",
        "ssh-ed25519 AAAA",
        "--usb-enable-dhcp",
    ]);

    assert_eq!(
        fixture.boot_file("sysconf.txt").unwrap(),
        "hostname=beagle\n\
         timezone=Asia/Kolkata\n\
         keymap=us\n\
         user_name=bob\n\
         user_password=hunter2\n\
         user_authorized_key=ssh-ed25519 AAAA\n\
         usb_enable_dhcp=yes\n"
    );
}

/// A single flag is enough to trigger customization; unset fields are omitted
/// rather than written empty.
#[test]
fn flash_sd_sysconfig_omits_unset_keys() {
    let mut fixture = SdFixture::new();

    run_cli([
        "bb-imager-cli",
        "flash",
        "--quiet",
        "sd",
        fixture.img(),
        fixture.dst(),
        "--file-destination",
        "--sysconfig",
        "--hostname",
        "beagle",
    ]);

    assert_eq!(
        fixture.boot_file("sysconf.txt").unwrap(),
        "hostname=beagle\n"
    );
}

/// `--usb-enable-dhcp` is a bool, so it is only meaningful as a positive
/// assertion: leaving it off must not emit the key.
#[test]
fn flash_sd_without_usb_dhcp_flag_omits_the_key() {
    let mut fixture = SdFixture::new();

    run_cli([
        "bb-imager-cli",
        "flash",
        "--quiet",
        "sd",
        fixture.img(),
        fixture.dst(),
        "--file-destination",
        "--sysconfig",
        "--keymap",
        "us",
    ]);

    assert_eq!(fixture.boot_file("sysconf.txt").unwrap(), "keymap=us\n");
}

/// Wi-Fi credentials are split across two files: the sysconfig key points at a
/// per-SSID PSK file written under `services/`.
///
/// NOTE: `services/` must already exist in the image's boot partition — the
/// customization writer uses `create_file`, which does not create parent
/// directories, so `--wifi-ssid` on an image without that directory fails the
/// whole flash with "Failed to create customization services/<ssid>.psk".
#[test]
fn flash_sd_wifi_writes_psk_file_next_to_sysconfig() {
    let mut fixture = SdFixture::with_boot_dirs(&["services"]);

    run_cli([
        "bb-imager-cli",
        "flash",
        "--quiet",
        "sd",
        fixture.img(),
        fixture.dst(),
        "--file-destination",
        "--sysconfig",
        "--wifi-ssid",
        "mynet",
        "--wifi-password",
        "hunter2",
    ]);

    assert_eq!(
        fixture.boot_file("sysconf.txt").unwrap(),
        "iwd_psk_file=mynet.psk\n"
    );
    assert_eq!(
        fixture.boot_file("services/mynet.psk").unwrap(),
        "[Security]\nPassphrase=hunter2\n\n[Settings]\nAutoConnect=true"
    );
}

/// `--cloud-init` adds a cloud-init document *in addition to* sysconfig: the
/// CLI still generates sysconfig unconditionally (see the fallback TODO in
/// `flash_internal`). This test pins that current behaviour.
#[test]
fn flash_sd_cloud_init_emits_both_configs() {
    let mut fixture = SdFixture::new();

    run_cli([
        "bb-imager-cli",
        "flash",
        "--quiet",
        "sd",
        fixture.img(),
        fixture.dst(),
        "--file-destination",
        "--cloud-init",
        "--hostname",
        "beagle",
        "--user-name",
        "bob",
        "--user-password",
        "hunter2",
    ]);

    let cloud_init = fixture.boot_file("cloud-init").unwrap();
    assert!(
        cloud_init.starts_with("#cloud-config\n"),
        "cloud-init must carry the cloud-config header, got: {cloud_init}"
    );
    assert!(
        cloud_init.contains("beagle"),
        "cloud-init should carry the hostname, got: {cloud_init}"
    );

    assert!(
        fixture.boot_file("sysconf.txt").is_ok(),
        "sysconfig is still generated alongside cloud-init"
    );
}

/// Customization is only applied to images with a readable boot partition;
/// a raw payload has no partition table, so the flash must fail loudly rather
/// than silently dropping the requested config.
#[test]
#[should_panic(expected = "Failed to flash")]
fn flash_sd_customization_on_partitionless_image_fails() {
    let img = pattern_file(64 * 1024);
    let dst = NamedTempFile::new().unwrap();

    run_cli([
        "bb-imager-cli",
        "flash",
        "--quiet",
        "sd",
        img.path().to_str().unwrap(),
        dst.path().to_str().unwrap(),
        "--file-destination",
        "--sysconfig",
        "--hostname",
        "beagle",
    ]);
}

#[test]
#[should_panic(expected = "Failed to flash")]
fn flash_sd_missing_image_fails() {
    let dst = NamedTempFile::new().unwrap();

    run_cli([
        "bb-imager-cli",
        "flash",
        "--quiet",
        "sd",
        "/nonexistent/image.img",
        dst.path().to_str().unwrap(),
        "--file-destination",
    ]);
}

/// `--bmap` is resolved lazily through `LocalStringFile`; an unreadable path
/// must abort the flash rather than fall back to a full copy.
#[test]
#[should_panic(expected = "Failed to flash")]
fn flash_sd_unreadable_bmap_fails() {
    let img = pattern_file(4 * 1024);
    let dst = NamedTempFile::new().unwrap();

    run_cli([
        "bb-imager-cli",
        "flash",
        "--quiet",
        "sd",
        img.path().to_str().unwrap(),
        dst.path().to_str().unwrap(),
        "--file-destination",
        "--bmap",
        "/nonexistent/image.bmap",
    ]);
}

/// Build a tar archive containing `entries`, as consumed by `sd-boot-update`.
fn tar_archive(entries: &[(&str, &str)]) -> NamedTempFile {
    let file = NamedTempFile::new().unwrap();
    {
        let mut builder = tar::Builder::new(file.reopen().unwrap());
        for (name, contents) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(contents.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, name, contents.as_bytes())
                .unwrap();
        }
        builder.finish().unwrap();
    }
    file
}

#[test]
fn sd_boot_update_writes_archive_into_boot_partition() {
    let mut fixture = SdFixture::new();
    let archive = tar_archive(&[("extlinux.conf", "LABEL Linux\n")]);

    run_cli([
        "bb-imager-cli",
        "flash",
        "--quiet",
        "sd-boot-update",
        archive.path().to_str().unwrap(),
        fixture.dst(),
    ]);

    assert_eq!(fixture.boot_file("extlinux.conf").unwrap(), "LABEL Linux\n");
}

/// The non-quiet path for `sd-boot-update` runs its own progress-forwarding
/// thread (which rewrites archive progress into `FlashingProgress`), so it
/// needs separate coverage from the quiet path.
#[test]
fn sd_boot_update_renders_progress_when_not_quiet() {
    let mut fixture = SdFixture::new();
    let archive = tar_archive(&[("uEnv.txt", "console=ttyS2\n")]);

    run_cli([
        "bb-imager-cli",
        "flash",
        "sd-boot-update",
        archive.path().to_str().unwrap(),
        fixture.dst(),
    ]);

    assert_eq!(fixture.boot_file("uEnv.txt").unwrap(), "console=ttyS2\n");
}

#[test]
#[should_panic(expected = "Failed to flash")]
fn sd_boot_update_missing_archive_fails() {
    let fixture = SdFixture::new();

    run_cli([
        "bb-imager-cli",
        "flash",
        "--quiet",
        "sd-boot-update",
        "/nonexistent/boot.tar",
        fixture.dst(),
    ]);
}
