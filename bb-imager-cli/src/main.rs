mod cli;

use bb_imager::{BBFlasher, DownloadFlashingStatus, img::LocalImage};
use clap::{CommandFactory, Parser};
use cli::{Commands, DestinationsTarget, Opt, TargetCommands};
use futures::StreamExt;
use std::{ffi::CString, path::PathBuf};

#[tokio::main]
async fn main() {
    let opt = Opt::parse();

    match opt.command {
        Commands::Flash { target, quiet } => flash(*target, quiet).await,
        Commands::Format { dst, quiet } => format(dst, quiet).await,
        Commands::ListDestinations { target, no_frills } => {
            list_destinations(target, no_frills).await;
        }
        Commands::GenerateCompletion { shell } => generate_completion(shell),
    }
}

async fn flash(target: TargetCommands, quite: bool) {
    if quite {
        flash_internal(target, None).await
    } else {
        let (tx, mut rx) = futures::channel::mpsc::channel(20);
        tokio::task::spawn(async move {
            let term = console::Term::stdout();
            let bar_style =
                indicatif::ProgressStyle::with_template("{msg:15}  [{wide_bar}] [{percent:3} %]")
                    .expect("Failed to create progress bar");
            let bars = indicatif::MultiProgress::new();

            let mut last_bar: Option<indicatif::ProgressBar> = None;
            let mut last_state = DownloadFlashingStatus::Preparing;
            let mut stage = 1;

            // Setting initial stage as Preparing
            term.write_line(&stage_msg(DownloadFlashingStatus::Preparing, stage))
                .unwrap();

            while let Some(progress) = rx.next().await {
                // Skip if no change in stage
                if progress == last_state {
                    continue;
                }

                match (progress, last_state) {
                    // Take care when just progress needs to be updated
                    (
                        DownloadFlashingStatus::DownloadingProgress(p),
                        DownloadFlashingStatus::DownloadingProgress(_),
                    )
                    | (
                        DownloadFlashingStatus::FlashingProgress(p),
                        DownloadFlashingStatus::FlashingProgress(_),
                    )
                    | (
                        DownloadFlashingStatus::VerifyingProgress(p),
                        DownloadFlashingStatus::VerifyingProgress(_),
                    ) => {
                        last_bar.as_ref().unwrap().set_position((p * 100.0) as u64);
                    }
                    // Create new bar when stage has changed
                    (DownloadFlashingStatus::DownloadingProgress(p), _)
                    | (DownloadFlashingStatus::VerifyingProgress(p), _)
                    | (DownloadFlashingStatus::FlashingProgress(p), _) => {
                        if let Some(b) = last_bar.take() {
                            b.finish();
                        }

                        stage += 1;

                        let temp_bar = bars.add(indicatif::ProgressBar::new(100));
                        temp_bar.set_style(bar_style.clone());
                        temp_bar.set_message(stage_msg(progress, stage));
                        temp_bar.set_position((p * 100.0) as u64);
                        last_bar = Some(temp_bar);
                    }
                    // Print stage when entering a new stage without progress
                    (DownloadFlashingStatus::Verifying, _)
                    | (DownloadFlashingStatus::Customizing, _)
                    | (DownloadFlashingStatus::Preparing, _) => {
                        if let Some(b) = last_bar.take() {
                            b.finish();
                        }

                        stage += 1;
                        term.write_line(&stage_msg(progress, stage)).unwrap();
                    }
                }

                last_state = progress;
            }

            if let Some(b) = last_bar.take() {
                b.finish();
            }
        });

        flash_internal(target, Some(tx)).await
    }
    .expect("Filed to flash")
}

async fn flash_internal(
    target: TargetCommands,
    chan: Option<futures::channel::mpsc::Sender<bb_imager::DownloadFlashingStatus>>,
) -> std::io::Result<()> {
    match target {
        TargetCommands::Bcf {
            img,
            dst,
            no_verify,
        } => {
            let customization = bb_imager::flasher::FlashingBcfConfig { verify: !no_verify };
            bb_imager::flasher::bcf::BeagleConnectFreedom::new(
                LocalImage::new(img),
                dst,
                customization,
            )
            .flash(chan)
            .await
        }
        TargetCommands::Sd {
            dst,
            no_verify,
            hostname,
            timezone,
            keymap,
            user_name,
            user_password,
            wifi_ssid,
            wifi_password,
            img,
        } => {
            let user = user_name.map(|x| (x, user_password.unwrap()));
            let wifi = wifi_ssid.map(|x| (x, wifi_password.unwrap()));

            let customization = bb_imager::flasher::FlashingSdLinuxConfig::default()
                .update_verify(!no_verify)
                .update_hostname(hostname)
                .update_timezone(timezone)
                .update_keymap(keymap)
                .update_user(user)
                .update_wifi(wifi);

            bb_imager::flasher::sd::LinuxSd::new(LocalImage::new(img), dst, customization)
                .flash(chan)
                .await
        }
        TargetCommands::Msp430 { img, dst } => {
            bb_imager::flasher::msp430::Msp430::new(
                LocalImage::new(img),
                CString::new(dst).expect("Failed to parse destination"),
            )
            .flash(chan)
            .await
        }
        #[cfg(feature = "pb2_mspm0")]
        TargetCommands::Pb2Mspm0 { no_eeprom, img } => {
            bb_imager::flasher::pb2_mspm0::Pb2Mspm0::new(LocalImage::new(img), !no_eeprom)
                .flash(chan)
                .await
        }
    }
}

async fn format(dst: PathBuf, quite: bool) {
    let (tx, _) = futures::channel::mpsc::channel(20);
    let term = console::Term::stdout();

    let config = bb_imager::flasher::sd::LinuxSdFormat::new(dst);
    config.flash(Some(tx)).await.unwrap();

    if !quite {
        term.write_line("Formatting successful").unwrap();
    }
}

async fn list_destinations(target: DestinationsTarget, no_frills: bool) {
    let term = console::Term::stdout();

    let dsts = bb_imager::Flasher::from(target).destinations().await;

    if no_frills {
        for d in dsts {
            term.write_line(&d.path().to_string_lossy()).unwrap();
        }
        return;
    }

    match target {
        DestinationsTarget::Sd => {
            const NAME_HEADER: &str = "SD Card";
            const PATH_HEADER: &str = "Path";
            const SIZE_HEADER: &str = "Size (in G)";
            const BYTES_IN_GB: u64 = 1024 * 1024 * 1024;

            let dsts_str: Vec<_> = dsts
                .into_iter()
                .map(|x| {
                    (
                        x.to_string().trim().to_string(),
                        x.path().to_string_lossy().to_string(),
                        (x.size() / BYTES_IN_GB).to_string(),
                    )
                })
                .collect();

            let max_name_len = dsts_str
                .iter()
                .map(|x| x.0.len())
                .chain([NAME_HEADER.len()])
                .max()
                .unwrap();
            let max_path_len = dsts_str
                .iter()
                .map(|x| x.1.len())
                .chain([PATH_HEADER.len()])
                .max()
                .unwrap();
            let max_size_len = dsts_str
                .iter()
                .map(|x| x.2.len())
                .chain([SIZE_HEADER.len()])
                .max()
                .unwrap();

            let table_border = format!(
                "+-{}-+-{}-+-{}-+",
                std::iter::repeat_n('-', max_name_len).collect::<String>(),
                std::iter::repeat_n('-', max_path_len).collect::<String>(),
                std::iter::repeat_n('-', SIZE_HEADER.len()).collect::<String>(),
            );

            term.write_line(&table_border).unwrap();

            term.write_line(&format!(
                "| {} | {} | {: <6} |",
                console::pad_str(NAME_HEADER, max_name_len, console::Alignment::Left, None),
                console::pad_str(PATH_HEADER, max_path_len, console::Alignment::Left, None),
                console::pad_str(SIZE_HEADER, max_size_len, console::Alignment::Left, None),
            ))
            .unwrap();

            term.write_line(&table_border).unwrap();

            for d in dsts_str {
                term.write_line(&format!(
                    "| {} | {} | {} |",
                    console::pad_str(&d.0, max_name_len, console::Alignment::Left, None),
                    console::pad_str(&d.1, max_path_len, console::Alignment::Left, None),
                    console::pad_str(&d.2, max_size_len, console::Alignment::Right, None)
                ))
                .unwrap();
            }

            term.write_line(&table_border).unwrap();
        }
        _ => {
            for d in dsts {
                term.write_line(d.to_string().as_str()).unwrap();
            }
        }
    }
}

const fn progress_msg(status: DownloadFlashingStatus) -> &'static str {
    match status {
        DownloadFlashingStatus::Preparing => "Preparing  ",
        DownloadFlashingStatus::DownloadingProgress(_) => "Downloading",
        DownloadFlashingStatus::FlashingProgress(_) => "Flashing",
        DownloadFlashingStatus::Verifying | DownloadFlashingStatus::VerifyingProgress(_) => {
            "Verifying"
        }
        DownloadFlashingStatus::Customizing => "Customizing",
    }
}

fn stage_msg(status: DownloadFlashingStatus, stage: usize) -> String {
    format!("[{stage}] {}", progress_msg(status))
}

fn generate_completion(target: clap_complete::Shell) {
    let mut cmd = Opt::command();
    const BIN_NAME: &str = env!("CARGO_PKG_NAME");

    clap_complete::generate(target, &mut cmd, BIN_NAME, &mut std::io::stdout())
}

impl From<DestinationsTarget> for bb_imager::Flasher {
    fn from(value: DestinationsTarget) -> Self {
        match value {
            DestinationsTarget::Bcf => Self::BeagleConnectFreedom,
            DestinationsTarget::Sd => Self::SdCard,
            DestinationsTarget::Msp430 => Self::Msp430Usb,
            #[cfg(feature = "pb2_mspm0")]
            DestinationsTarget::Pb2Mspm0 => Self::Pb2Mspm0,
        }
    }
}
