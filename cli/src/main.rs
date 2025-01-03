use bb_imager::DownloadFlashingStatus;
use clap::{Parser, Subcommand, ValueEnum};
use std::{
    ffi::CString,
    path::PathBuf,
    sync::{Once, OnceLock},
};

#[derive(Parser)]
struct Opt {
    #[command(subcommand)]
    command: Commands,
    #[arg(long)]
    quite: bool,
}

#[derive(Subcommand)]
enum Commands {
    Flash {
        img: PathBuf,
        dst: String,
        #[command(subcommand)]
        target: TargetCommands,
    },
    ListDestinations {
        target: DestinationsTarget,
    },
}

#[derive(Subcommand)]
enum TargetCommands {
    Bcf {
        #[arg(long)]
        no_verify: bool,
    },
    Sd {
        #[arg(long)]
        no_verify: bool,
        #[arg(long)]
        hostname: Option<String>,
        #[arg(long)]
        timezone: Option<String>,
        #[arg(long)]
        keymap: Option<String>,
        #[arg(long, requires = "user_password")]
        user_name: Option<String>,
        #[arg(long, requires = "user_name")]
        user_password: Option<String>,
        #[arg(long, requires = "wifi_password")]
        wifi_ssid: Option<String>,
        #[arg(long, requires = "wifi_ssid")]
        wifi_password: Option<String>,
    },
    Msp430,
}

#[derive(ValueEnum, Clone, Copy)]
enum DestinationsTarget {
    Bcf,
    Sd,
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
