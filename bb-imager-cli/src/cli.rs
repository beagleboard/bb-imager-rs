use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Opt {
    #[command(subcommand)]
    /// Specifies the subcommand to execute.
    pub command: Commands,
    #[arg(long)]
    /// Enable more logging.
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Command to flash an image to a specific destination.
    Flash {
        #[command(subcommand)]
        /// Type of BeagleBoard to flash
        target: Box<TargetCommands>,

        #[arg(long)]
        /// Suppress standard output messages for a quieter experience.
        quiet: bool,
    },

    /// Command to list available destinations for flashing based on the selected target.
    ListDestinations {
        /// Specifies the target type for listing destinations.
        target: DestinationsTarget,

        #[arg(long)]
        /// Only print paths seperated by newline
        no_frills: bool,

        #[arg(long)]
        /// Show all possible destinations without any sanity filters. Can be used when a device is
        /// not visible due to incorrect reporting by OS.
        no_filter: bool,
    },

    /// Command to format SD Card
    Format {
        /// The destination device (e.g., `/dev/sdX` or specific device identifiers).
        dst: PathBuf,

        #[arg(long)]
        /// Suppress standard output messages for a quieter experience.
        quiet: bool,
    },

    /// Command to generate shell completion
    GenerateCompletion {
        /// Specifies the target shell type for completion
        shell: clap_complete::Shell,
    },
}

#[derive(Subcommand, Debug)]
pub enum TargetCommands {
    /// Flash BeagleConnect Freedom.
    #[cfg(feature = "bcf_cc1352p7")]
    Bcf {
        /// Local path to image file. Can be compressed (xz) or extracted file
        img: Box<Path>,

        /// The destination device (e.g., `/dev/sdX` or specific device identifiers).
        dst: String,

        #[arg(long)]
        /// Disable checksum verification after flashing to speed up the process.
        no_verify: bool,
    },
    /// Flash an SD card with customizable settings for BeagleBoard devices.
    Sd {
        /// Local path to image file. Can be compressed (xz) or extracted file
        img: Box<Path>,

        /// The destination device (e.g., `/dev/sdX` or specific device identifiers).
        dst: PathBuf,

        #[arg(long)]
        /// Set a custom hostname for the device (e.g., "beaglebone").
        hostname: Option<Box<str>>,

        #[arg(long)]
        /// Set the timezone for the device (e.g., "America/New_York").
        timezone: Option<Box<str>>,

        #[arg(long)]
        /// Set the keyboard layout/keymap (e.g., "us" for the US layout).
        keymap: Option<Box<str>>,

        #[arg(long, requires = "user_password", verbatim_doc_comment)]
        /// Set a username for the default user. Cannot be `root`. Requires `user_password`.
        /// Required to enter GUI session due to regulatory requirements.
        user_name: Option<Box<str>>,

        #[arg(long, requires = "user_name", verbatim_doc_comment)]
        /// Set a password for the default user. Requires `user_name`.
        /// Required to enter GUI session due to regulatory requirements.
        user_password: Option<Box<str>>,

        #[arg(long, requires = "wifi_password")]
        /// Configure a Wi-Fi SSID for network access. Requires `wifi_password`.
        wifi_ssid: Option<Box<str>>,

        #[arg(long, requires = "wifi_ssid")]
        /// Set the password for the specified Wi-Fi SSID. Requires `wifi_ssid`.
        wifi_password: Option<Box<str>>,

        #[arg(long)]
        /// Set SSH public key for authentication
        ssh_key: Option<Box<str>>,

        #[arg(long)]
        /// Enable USB DHCP
        usb_enable_dhcp: bool,
        /// Provide the bmap file for the image
        #[arg(long)]
        bmap: Option<Box<Path>>,

        #[arg(long)]
        /// Generate clound-init config.
        cloud_init: bool,

        #[arg(long)]
        /// Generate sysconfig. Currently, sysconfig will be generated regardless if this flag is
        /// provides. However, this will change in future. So best to explicitly set the flag.
        sysconfig: bool,

        /// The destination is a file instead of SD Card
        #[arg(long)]
        file_destination: bool
    },
    /// Update boot partition with contents from archive
    SdBootUpdate {
        /// Local path to bootfs archive.
        img: Box<Path>,

        /// The destination device (e.g., `/dev/sdX` or specific device identifiers).
        dst: PathBuf,
    },
    /// Flash MSP430 on BeagleConnectFreedom.
    #[cfg(feature = "bcf_msp430")]
    Msp430 {
        /// Local path to image file. Can be compressed (xz) or extracted file
        img: Box<Path>,

        /// The destination device (e.g., `/dev/sdX` or specific device identifiers).
        dst: String,
    },
    /// Flash MSPM0 on Pocketbeagle2.
    #[cfg(feature = "pb2_mspm0")]
    Pb2Mspm0 {
        /// Local path to image file. Can be compressed (xz) or extracted file
        img: Box<Path>,

        /// Do not persist EEPROM contents
        #[arg(long)]
        no_eeprom: bool,
    },
    #[cfg(feature = "dfu")]
    Dfu {
        /// Identifer is in the following format: `{bus_num}:{address}:{vendor_id}:{product_id}`.
        /// All fields are in hex.
        identifier: String,
        /// Format {name} followed by {path}. Any number of firmware can be specified, which will
        /// be flashed in a sequential order.
        imgs: Vec<String>,
    },
    /// Flash Zepto
    #[cfg(any(feature = "zepto_uart", feature = "zepto_i2c"))]
    Zepto {
        /// Local path to image file. Can be compressed (xz) or extracted file
        img: Box<Path>,
        /// The destination device (e.g., `/dev/tty*` or `/dev/i2c-*` or specific device identifiers).
        dst: String,
        #[arg(long)]
        /// Disable checksum verification after flashing to speed up the process.
        no_verify: bool,
        #[cfg(target_os = "linux")]
        #[arg(long, requires = "bsl_gpio")]
        /// RESET GPIO for MSPM0.
        reset_gpio: Option<String>,
        #[cfg(target_os = "linux")]
        #[arg(long, requires = "reset_gpio")]
        /// BSL GPIO for MSPM0.
        bsl_gpio: Option<String>,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum DestinationsTarget {
    /// BeagleConnect Freedom targets.
    #[cfg(feature = "bcf_cc1352p7")]
    Bcf,
    /// SD card targets for BeagleBoard devices.
    Sd,
    /// MSP430 targets
    #[cfg(feature = "bcf_msp430")]
    Msp430,
    /// Pocketbeagle2 MSPM0
    #[cfg(feature = "pb2_mspm0")]
    Pb2Mspm0,
    /// USB DFU Target
    #[cfg(feature = "dfu")]
    Dfu,
    /// Zepto Target
    #[cfg(any(feature = "zepto_uart", feature = "zepto_i2c"))]
    Zepto,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    /// clap's own recommended smoke test: validates the entire derived command
    /// tree (no duplicate args, well-formed `requires`/`conflicts`, etc.).
    #[test]
    fn cli_definition_is_valid() {
        Opt::command().debug_assert();
    }

    #[test]
    fn flash_sd_minimal_parses() {
        let opt = Opt::try_parse_from(["bb-imager-cli", "flash", "sd", "img.xz", "/dev/sdX"])
            .expect("valid sd flash invocation");
        assert!(!opt.verbose);
        match opt.command {
            Commands::Flash { target, quiet } => {
                assert!(!quiet);
                match *target {
                    TargetCommands::Sd { img, dst, .. } => {
                        assert_eq!(img.as_ref(), Path::new("img.xz"));
                        assert_eq!(dst, PathBuf::from("/dev/sdX"));
                    }
                    other => panic!("expected Sd, got {other:?}"),
                }
            }
            other => panic!("expected Flash, got {other:?}"),
        }
    }

    #[test]
    fn flash_sd_customization_flags_parse() {
        let opt = Opt::try_parse_from([
            "bb-imager-cli",
            "flash",
            "sd",
            "img.xz",
            "/dev/sdX",
            "--hostname",
            "beagle",
            "--usb-enable-dhcp",
            "--file-destination",
        ])
        .expect("valid customized sd flash");
        match opt.command {
            Commands::Flash { target, .. } => match *target {
                TargetCommands::Sd {
                    hostname,
                    usb_enable_dhcp,
                    file_destination,
                    ..
                } => {
                    assert_eq!(hostname.as_deref(), Some("beagle"));
                    assert!(usb_enable_dhcp);
                    assert!(file_destination);
                }
                other => panic!("expected Sd, got {other:?}"),
            },
            other => panic!("expected Flash, got {other:?}"),
        }
    }

    #[test]
    fn user_name_requires_password() {
        // `--user-name` declares `requires = "user_password"`.
        assert!(
            Opt::try_parse_from([
                "bb-imager-cli",
                "flash",
                "sd",
                "i",
                "/d",
                "--user-name",
                "bob",
            ])
            .is_err()
        );
        assert!(
            Opt::try_parse_from([
                "bb-imager-cli",
                "flash",
                "sd",
                "i",
                "/d",
                "--user-name",
                "bob",
                "--user-password",
                "pw",
            ])
            .is_ok()
        );
    }

    #[test]
    fn wifi_ssid_requires_password() {
        assert!(
            Opt::try_parse_from([
                "bb-imager-cli",
                "flash",
                "sd",
                "i",
                "/d",
                "--wifi-ssid",
                "net",
            ])
            .is_err()
        );
        assert!(
            Opt::try_parse_from([
                "bb-imager-cli",
                "flash",
                "sd",
                "i",
                "/d",
                "--wifi-ssid",
                "net",
                "--wifi-password",
                "pw",
            ])
            .is_ok()
        );
    }

    #[test]
    fn list_destinations_flags_parse() {
        let opt = Opt::try_parse_from([
            "bb-imager-cli",
            "list-destinations",
            "sd",
            "--no-frills",
            "--no-filter",
        ])
        .expect("valid list-destinations");
        match opt.command {
            Commands::ListDestinations {
                target,
                no_frills,
                no_filter,
            } => {
                assert!(matches!(target, DestinationsTarget::Sd));
                assert!(no_frills);
                assert!(no_filter);
            }
            other => panic!("expected ListDestinations, got {other:?}"),
        }
    }

    #[test]
    fn format_and_verbose_parse() {
        let opt =
            Opt::try_parse_from(["bb-imager-cli", "--verbose", "format", "/dev/sdX", "--quiet"])
                .expect("valid format invocation");
        assert!(opt.verbose);
        match opt.command {
            Commands::Format { dst, quiet } => {
                assert_eq!(dst, PathBuf::from("/dev/sdX"));
                assert!(quiet);
            }
            other => panic!("expected Format, got {other:?}"),
        }
    }

    #[test]
    fn generate_completion_parses_shell() {
        let opt = Opt::try_parse_from(["bb-imager-cli", "generate-completion", "bash"])
            .expect("valid completion invocation");
        assert!(matches!(
            opt.command,
            Commands::GenerateCompletion {
                shell: clap_complete::Shell::Bash
            }
        ));
    }

    #[test]
    fn unknown_subcommand_is_rejected() {
        assert!(Opt::try_parse_from(["bb-imager-cli", "bogus"]).is_err());
    }

    #[test]
    fn sd_boot_update_parses() {
        let opt =
            Opt::try_parse_from(["bb-imager-cli", "flash", "sd-boot-update", "boot.tar", "/dev/sdX"])
                .expect("valid sd-boot-update");
        match opt.command {
            Commands::Flash { target, .. } => match *target {
                TargetCommands::SdBootUpdate { img, dst } => {
                    assert_eq!(img.as_ref(), Path::new("boot.tar"));
                    assert_eq!(dst, PathBuf::from("/dev/sdX"));
                }
                other => panic!("expected SdBootUpdate, got {other:?}"),
            },
            other => panic!("expected Flash, got {other:?}"),
        }
    }

    #[cfg(feature = "pb2_mspm0")]
    #[test]
    fn pb2_mspm0_variant_parses() {
        let opt = Opt::try_parse_from([
            "bb-imager-cli",
            "flash",
            "pb2-mspm0",
            "fw.bin",
            "--no-eeprom",
        ])
        .expect("valid pb2-mspm0 flash");
        match opt.command {
            Commands::Flash { target, .. } => match *target {
                TargetCommands::Pb2Mspm0 { no_eeprom, img } => {
                    assert!(no_eeprom);
                    assert_eq!(img.as_ref(), Path::new("fw.bin"));
                }
                other => panic!("expected Pb2Mspm0, got {other:?}"),
            },
            other => panic!("expected Flash, got {other:?}"),
        }
    }
}
