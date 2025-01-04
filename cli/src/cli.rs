use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Opt {
    #[command(subcommand)]
    /// Specifies the subcommand to execute.
    pub command: Commands,

    #[arg(long)]
    /// Suppress standard output messages for a quieter experience.
    pub quite: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Command to flash an image to a specific destination.
    Flash {
        /// The destination device (e.g., `/dev/sdX` or specific device identifiers).
        dst: String,

        #[arg(group = "image")]
        /// Path to the image file to flash. Supports both raw and compressed (e.g., xz) formats.
        img: Option<PathBuf>,

        #[arg(long, group = "image")]
        /// URL to remote image file to flash. Supports both raw and compressed (e.g., xz) formats.
        image_remote: Option<url::Url>,

        #[arg(long, requires = "image_remote")]
        /// Checksum for remote image.
        image_sha256: Option<String>,

        #[command(subcommand)]
        /// Type of BeagleBoard to flash
        target: TargetCommands,
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
        dst: String,
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
        #[arg(long)]
        /// Disable checksum verification after flashing to speed up the process.
        no_verify: bool,
    },
    /// Flash an SD card with customizable settings for BeagleBoard devices.
    Sd {
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
    Msp430,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum DestinationsTarget {
    /// BeagleConnect Freedom targets.
    Bcf,
    /// SD card targets for BeagleBoard devices.
    Sd,
    /// MSP430 targets
    Msp430,
}
