use bb_imager::DownloadFlashingStatus;
use clap::{Parser, Subcommand, ValueEnum};
use std::{
    path::PathBuf,
    sync::{Once, OnceLock},
};

#[derive(Parser)]
struct Opt {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Flash {
        img: PathBuf,
        dst: String,
        target: FlashTarget,
    },
}

#[derive(ValueEnum, Clone, Copy)]
enum FlashTarget {
    Bcf,
    Sd,
}

impl From<FlashTarget> for bb_imager::config::Flasher {
    fn from(value: FlashTarget) -> Self {
        match value {
            FlashTarget::Bcf => Self::BeagleConnectFreedom,
            FlashTarget::Sd => Self::SdCard,
        }
    }
}

#[tokio::main]
async fn main() {
    let opt = Opt::parse();

    match opt.command {
        Commands::Flash { img, dst, target } => {
            let state = bb_imager::State::new().await.unwrap();
            let downloader = bb_imager::download::Downloader::new();
            let (tx, rx) = std::sync::mpsc::channel();
            let dst = match target {
                FlashTarget::Bcf => bb_imager::Destination::port(dst),
                FlashTarget::Sd => bb_imager::Destination::from_path(&dst, &state)
                    .await
                    .unwrap(),
            };

            tokio::task::spawn_blocking(move || {
                let bars = indicatif::MultiProgress::new();
                static FLASHING: OnceLock<indicatif::ProgressBar> = OnceLock::new();
                static VERIFYING: OnceLock<indicatif::ProgressBar> = OnceLock::new();

                while let Ok(progress) = rx.recv() {
                    match progress {
                        DownloadFlashingStatus::Preparing => {
                            static PREPARING: Once = Once::new();

                            PREPARING.call_once(|| {
                                println!("[1/3] Preparing");
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
                                        "[2/3] {msg}  [{wide_bar}] [{percent} %]",
                                    )
                                    .unwrap(),
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

                            VERIFYING.call_once(|| println!("[3/3] Verifying"));
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
                                        "[3/3] {msg} [{wide_bar}] [{percent} %]",
                                    )
                                    .unwrap(),
                                );
                                bar.set_message("Verifying");
                                bar
                            });

                            bar.set_position((p * 100.0) as u64);
                        }
                        DownloadFlashingStatus::Finished => {
                            if let Some(x) = VERIFYING.get() {
                                if !x.is_finished() {
                                    x.finish()
                                }
                            }
                        }
                    };
                }
            });

            bb_imager::download_and_flash(
                bb_imager::SelectedImage::local(img),
                dst,
                target.into(),
                state,
                downloader,
                tx,
            )
            .await
            .unwrap();
        }
    }

    // bb_imager::bcf::flash(&opt.img, opt.dst).unwrap();

    // if !opt.img.exists() {
    //     eprintln!("Provided Image does not exist {:?}", opt.img);
    //     return;
    // }

    // if !opt.dst.exists() {
    //     eprintln!("Provided destination does not exist {:?}", opt.dst);
    //     return;
    // }

    // bb_imager::format(&opt.dst).expect("Failed to format disk");
    // flash(&opt.img, &opt.dst).expect("Failed to flash");
}
