use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

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
    #[cfg(feature = "bcf_cc1352p7")]
    Bcf {
        /// Local path to image file. Can be compressed (xz) or extracted file
        img: PathBuf,

        /// The destination device (e.g., `/dev/sdX` or specific device identifiers).
        dst: String,

        #[arg(long)]
        /// Disable checksum verification after flashing to speed up the process.
        no_verify: bool,
    },
    /// Flash an SD card with customizable settings for BeagleBoard devices.
    Sd {
        /// Local path to image file. Can be compressed (xz) or extracted file
        img: PathBuf,

        /// The destination device (e.g., `/dev/sdX` or specific device identifiers).
        dst: PathBuf,

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
        /// Set a username for the default user. Cannot be `root`. Requires `user_password`.
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

        #[arg(long)]
        /// Set SSH public key for authentication
        ssh_key: Option<String>,

        #[arg(long)]
        /// Enable USB DHCP
        usb_enable_dhcp: bool,
        /// Provide the bmap file for the image
        #[arg(long)]
        bmap: Option<PathBuf>,
    },
    /// Flash MSP430 on BeagleConnectFreedom.
    #[cfg(feature = "bcf_msp430")]
    Msp430 {
        /// Local path to image file. Can be compressed (xz) or extracted file
        img: PathBuf,

        /// The destination device (e.g., `/dev/sdX` or specific device identifiers).
        dst: String,
    },
    /// Flash MSPM0 on Pocketbeagle2.
    #[cfg(feature = "pb2_mspm0")]
    Pb2Mspm0 {
        /// Local path to image file. Can be compressed (xz) or extracted file
        img: PathBuf,

        /// Do not persist EEPROM contents
        #[arg(long)]
        no_eeprom: bool,
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
}
