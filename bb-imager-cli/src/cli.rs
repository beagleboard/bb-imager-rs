use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use url::Url;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Opt {
    #[command(subcommand)]
    /// Specifies the subcommand to execute.
    pub command: Commands,
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
    Bcf {
        #[command(flatten)]
        img: SelectedImage,

        /// The destination device (e.g., `/dev/sdX` or specific device identifiers).
        dst: String,

        #[arg(long)]
        /// Disable checksum verification after flashing to speed up the process.
        no_verify: bool,
    },
    /// Flash an SD card with customizable settings for BeagleBoard devices.
    Sd {
        #[command(flatten)]
        img: SelectedImage,

        /// The destination device (e.g., `/dev/sdX` or specific device identifiers).
        dst: PathBuf,

        /// Disable checksum verification post-flash
        #[arg(long)]
        no_verify: bool,

        #[arg(long)]
        /// Set a custom hostname for the device (e.g., "beaglebone").
        hostname: Option<String>,

        #[arg(long)]
        /// Set the timezone for the device (e.g., "America/New_York").
        timezone: Option<String>,

        #[arg(long)]
        /// Set the keyboard layout/keymap (e.g., "us" for the US layout).
        keymap: Option<String>,

        #[arg(long, requires = "user_password", verbatim_doc_comment)]
        /// Set a username for the default user. Requires `user_password`.
        /// Required to enter GUI session due to regulatory requirements.
        user_name: Option<String>,

        #[arg(long, requires = "user_name", verbatim_doc_comment)]
        /// Set a password for the default user. Requires `user_name`.
        /// Required to enter GUI session due to regulatory requirements.
        user_password: Option<String>,

        #[arg(long, requires = "wifi_password")]
        /// Configure a Wi-Fi SSID for network access. Requires `wifi_password`.
        wifi_ssid: Option<String>,

        #[arg(long, requires = "wifi_ssid")]
        /// Set the password for the specified Wi-Fi SSID. Requires `wifi_ssid`.
        wifi_password: Option<String>,
    },
    /// Flash MSP430 on BeagleConnectFreedom.
    Msp430 {
        #[command(flatten)]
        img: SelectedImage,

        /// The destination device (e.g., `/dev/sdX` or specific device identifiers).
        dst: String,
    },
    /// Flash MSPM0 on Pocketbeagle2.
    #[cfg(feature = "pb2_mspm0")]
    Pb2Mspm0 {
        #[command(flatten)]
        img: SelectedImage,

        /// Do not persist EEPROM contents
        #[arg(long)]
        no_eeprom: bool,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum DestinationsTarget {
    /// BeagleConnect Freedom targets.
    Bcf,
    /// SD card targets for BeagleBoard devices.
    Sd,
    /// MSP430 targets
    Msp430,
    /// Pocketbeagle2 MSPM0
    #[cfg(feature = "pb2_mspm0")]
    Pb2Mspm0,
}

#[derive(Args, Debug)]
pub struct SelectedImage {
    #[command(flatten)]
    pub img: OsImage,
    #[arg(long, requires = "img_remote")]
    /// Checksum for remote image.
    pub img_sha256: Option<String>,
}

#[derive(Args, Debug)]
#[group(required = true, multiple = false)]
pub struct OsImage {
    #[arg(long)]
    /// Path to the image file to flash. Supports both raw and compressed (e.g., xz) formats.
    pub img_local: Option<PathBuf>,
    #[arg(long, requires = "img_sha256")]
    /// URL to remote image file to flash. Supports both raw and compressed (e.g., xz) formats.
    pub img_remote: Option<Url>,
}
