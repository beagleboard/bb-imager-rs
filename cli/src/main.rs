use bb_imager::DownloadFlashingStatus;
use clap::{Parser, Subcommand, ValueEnum};
use std::{
    ffi::CString,
    path::PathBuf,
    sync::{Once, OnceLock},
};

#[derive(Parser)]
#[command(version, about)]
struct Opt {
    #[command(subcommand)]
    /// Specifies the subcommand to execute.
    command: Commands,

    #[arg(long)]
    /// Suppress standard output messages for a quieter experience.
    quite: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Command to flash an image to a specific destination.
    Flash {
        /// Path to the image file to flash. Supports both raw and compressed (e.g., xz) formats.
        img: PathBuf,

        /// The destination device (e.g., `/dev/sdX` or specific device identifiers).
        dst: String,

        #[command(subcommand)]
        /// Type of BeagleBoard to flash
        target: TargetCommands,
    },

    /// Command to list available destinations for flashing based on the selected target.
    ListDestinations {
        /// Specifies the target type for listing destinations.
        target: DestinationsTarget,
    },
}

#[derive(Subcommand)]
enum TargetCommands {
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

#[derive(ValueEnum, Clone, Copy)]
enum DestinationsTarget {
    /// BeagleConnect Freedom targets.
    Bcf,
    /// SD card targets for BeagleBoard devices.
    Sd,
    /// MSP430 targets
    Msp430,
}

impl From<DestinationsTarget> for bb_imager::config::Flasher {
    fn from(value: DestinationsTarget) -> Self {
        match value {
            DestinationsTarget::Bcf => Self::BeagleConnectFreedom,
            DestinationsTarget::Sd => Self::SdCard,
            DestinationsTarget::Msp430 => Self::Msp430Usb,
        }
    }
}

#[tokio::main]
async fn main() {
    let opt = Opt::parse();

    match opt.command {
        Commands::Flash { img, dst, target } => flash(img, dst, target, opt.quite).await,
        Commands::ListDestinations { target } => {
            let dsts = bb_imager::config::Flasher::from(target)
                .destinations()
                .await;

            match target {
                DestinationsTarget::Sd => {
                    println!("| {: <12} | {: <12} |", "Sd Card", "Size (in G)");
                    println!("|--------------|--------------|");
                    for d in dsts {
                        println!(
                            "| {: <12} | {: <12} |",
                            d.path().to_str().unwrap(),
                            d.size() / (1024 * 1024 * 1024)
                        )
                    }
                }
                DestinationsTarget::Bcf | DestinationsTarget::Msp430 => {
                    for d in dsts {
                        println!("{}", d)
                    }
                }
            }
        }
    }
}

async fn flash(img: PathBuf, dst: String, target: TargetCommands, quite: bool) {
    let downloader = bb_imager::download::Downloader::new();
    let (tx, mut rx) = tokio::sync::mpsc::channel(20);

    if !quite {
        tokio::task::spawn(async move {
            let bars = indicatif::MultiProgress::new();
            static FLASHING: OnceLock<indicatif::ProgressBar> = OnceLock::new();
            static VERIFYING: OnceLock<indicatif::ProgressBar> = OnceLock::new();

            while let Some(progress) = rx.recv().await {
                match progress {
                    DownloadFlashingStatus::Preparing => {
                        static PREPARING: Once = Once::new();

                        PREPARING.call_once(|| {
                            println!("Preparing");
                        });
                    }
                    DownloadFlashingStatus::DownloadingProgress(_) => {
                        panic!("Not Supported");
                    }
                    DownloadFlashingStatus::FlashingProgress(p) => {
                        let bar = FLASHING.get_or_init(|| {
                            let bar = bars.add(indicatif::ProgressBar::new(100));
                            bar.set_style(
                                indicatif::ProgressStyle::with_template(
                                    "{msg}  [{wide_bar}] [{percent} %]",
                                )
                                .expect("Failed to create progress bar"),
                            );
                            bar.set_message("Flashing");
                            bar
                        });

                        bar.set_position((p * 100.0) as u64);
                    }
                    DownloadFlashingStatus::Verifying => {
                        static VERIFYING: Once = Once::new();

                        if let Some(x) = FLASHING.get() {
                            if !x.is_finished() {
                                x.finish()
                            }
                        }

                        VERIFYING.call_once(|| println!("Verifying"));
                    }
                    DownloadFlashingStatus::VerifyingProgress(p) => {
                        if let Some(x) = FLASHING.get() {
                            if !x.is_finished() {
                                x.finish()
                            }
                        }

                        let bar = VERIFYING.get_or_init(|| {
                            let bar = bars.add(indicatif::ProgressBar::new(100));
                            bar.set_style(
                                indicatif::ProgressStyle::with_template(
                                    "{msg} [{wide_bar}] [{percent} %]",
                                )
                                .expect("Failed to create progress bar"),
                            );
                            bar.set_message("Verifying");
                            bar
                        });

                        bar.set_position((p * 100.0) as u64);
                    }
                    DownloadFlashingStatus::Customizing => {
                        static CUSTOMIZING: Once = Once::new();

                        // Finish verifying progress if not already done
                        if let Some(x) = VERIFYING.get() {
                            if !x.is_finished() {
                                x.finish()
                            }
                        }

                        CUSTOMIZING.call_once(|| {
                            println!("Customizing");
                        });
                    }
                };
            }
        });
    }

    let img = bb_imager::SelectedImage::local(img);
    let flashing_config = match target {
        TargetCommands::Bcf { no_verify } => {
            let customization = bb_imager::FlashingBcfConfig { verify: !no_verify };
            bb_imager::FlashingConfig::BeagleConnectFreedom {
                img,
                port: dst,
                customization,
            }
        }
        TargetCommands::Sd {
            no_verify,
            hostname,
            timezone,
            keymap,
            user_name,
            user_password,
            wifi_ssid,
            wifi_password,
        } => {
            let user = user_name.map(|x| (x, user_password.unwrap()));
            let wifi = wifi_ssid.map(|x| (x, wifi_password.unwrap()));

            let customization = bb_imager::FlashingSdLinuxConfig {
                verify: !no_verify,
                hostname,
                timezone,
                keymap,
                user,
                wifi,
            };
            bb_imager::FlashingConfig::LinuxSd {
                img,
                dst,
                customization,
            }
        }
        TargetCommands::Msp430 => bb_imager::FlashingConfig::Msp430 {
            img,
            port: CString::new(dst).expect("Failed to parse destination"),
        },
    };

    flashing_config
        .download_flash_customize(downloader, tx)
        .await
        .expect("Failed to flash");
}
