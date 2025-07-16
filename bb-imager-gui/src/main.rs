#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{collections::HashSet, time::Duration};

use bb_config::config;
use constants::PACKAGE_QUALIFIER;
use helpers::{FlashingCustomization, ProgressBarState};
use iced::{Subscription, Task, futures::SinkExt, widget};
use message::BBImagerMessage;
use pages::Screen;
use tokio_stream::StreamExt;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod constants;
mod helpers;
mod message;
mod pages;
mod persistance;
mod ui;

fn main() -> iced::Result {
    let dirs = crate::helpers::project_dirs().unwrap();
    let log_file_p = dirs
        .cache_dir()
        .with_file_name("bb-imager.log")
        .with_file_name(format!(
            "{}.{}.{}.log",
            PACKAGE_QUALIFIER.0, PACKAGE_QUALIFIER.1, PACKAGE_QUALIFIER.2
        ));

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(std::fs::File::create(&log_file_p).unwrap()),
        )
        .try_init()
        .expect("Failed to register tracing_subscriber");

    tracing::info!("Logging to file at: {:?}", log_file_p);

    let icon = iced::window::icon::from_file_data(
        constants::WINDOW_ICON,
        Some(iced::advanced::graphics::image::image_rs::ImageFormat::Png),
    )
    .ok();
    assert!(icon.is_some());

    #[cfg(target_os = "macos")]
    // HACK: mac_notification_sys set application name (not an option in notify-rust)
    let _ = notify_rust::set_application("org.beagleboard.imagingutility");

    let app_config = persistance::GuiConfiguration::load().unwrap_or_default();

    let settings = iced::window::Settings {
        min_size: Some(constants::WINDOW_SIZE),
        size: constants::WINDOW_SIZE,
        ..Default::default()
    };

    iced::application(app_title, message::update, ui::view)
        .subscription(BBImager::subscription)
        .theme(BBImager::theme)
        .window(settings)
        .font(constants::FONT_REGULAR_BYTES)
        .font(constants::FONT_BOLD_BYTES)
        .default_font(constants::FONT_REGULAR)
        .run_with(move || BBImager::new(app_config))
}

fn app_title(_: &BBImager) -> String {
    if option_env!("PRE_RELEASE").is_some() {
        format!("{} (pre-release)", constants::APP_NAME)
    } else {
        format!("{} v{}", constants::APP_NAME, env!("CARGO_PKG_VERSION"))
    }
}

#[derive(Debug)]
struct BBImager {
    app_config: persistance::GuiConfiguration,
    boards: helpers::Boards,
    downloader: bb_downloader::Downloader,
    screen: Vec<Screen>,
    selected_board: Option<usize>,
    selected_image: Option<helpers::BoardImage>,
    selected_dst: Option<helpers::Destination>,
    destinations: Vec<helpers::Destination>,
    cancel_flashing: Option<iced::task::Handle>,
    customization: Option<FlashingCustomization>,

    timezones: widget::combo_box::State<String>,
    keymaps: widget::combo_box::State<String>,
}

impl BBImager {
    fn new(app_config: persistance::GuiConfiguration) -> (Self, Task<BBImagerMessage>) {
        let downloader = bb_downloader::Downloader::new(
            directories::ProjectDirs::from(
                PACKAGE_QUALIFIER.0,
                PACKAGE_QUALIFIER.1,
                PACKAGE_QUALIFIER.2,
            )
            .unwrap()
            .cache_dir()
            .to_path_buf(),
        )
        .unwrap();

        // Fetch old config
        let client = downloader.clone();
        let config_task = helpers::refresh_config_task(client, &Default::default());

        let mut ans = Self {
            app_config,
            downloader: downloader.clone(),
            timezones: widget::combo_box::State::new(
                constants::TIMEZONES.iter().map(|x| x.to_string()).collect(),
            ),
            keymaps: widget::combo_box::State::new(
                constants::KEYMAP_LAYOUTS
                    .iter()
                    .map(|x| x.to_string())
                    .collect(),
            ),
            screen: Vec::with_capacity(3),
            boards: Default::default(),
            selected_board: Default::default(),
            selected_image: Default::default(),
            selected_dst: Default::default(),
            destinations: Default::default(),
            cancel_flashing: Default::default(),
            customization: Default::default(),
        };

        ans.screen.push(Screen::Home);

        // Fetch all board images
        let board_image_task = ans.fetch_board_images();

        (ans, Task::batch([config_task, board_image_task]))
    }

    fn fetch_board_images(&self) -> Task<BBImagerMessage> {
        // Do not try downloading same image multiple times
        let icons: HashSet<url::Url> = self
            .boards
            .devices()
            .filter_map(|(_, dev)| dev.icon.clone())
            .collect();

        let tasks = icons.into_iter().map(|icon| {
            let downloader = self.downloader.clone();
            let icon_clone = icon.clone();
            Task::perform(
                async move { downloader.download_no_cache(icon_clone, None).await },
                move |p| match p {
                    Ok(_) => BBImagerMessage::Null,
                    Err(_) => {
                        tracing::warn!("Failed to fetch image {}", icon);
                        BBImagerMessage::Null
                    }
                },
            )
        });
        Task::batch(tasks)
    }

    fn push_page(&mut self, x: Screen) -> Task<BBImagerMessage> {
        self.screen.push(x.clone());

        if let Screen::ImageSelection(page) = x {
            let board = self.selected_board.unwrap();
            return self.fetch_remote_subitems(board, page.idx());
        }

        Task::none()
    }

    fn fetch_remote_subitems(&self, board: usize, target: &[usize]) -> Task<BBImagerMessage> {
        let Some(os_images) = self.boards.images(board, target) else {
            // Maybe resolving was missed
            if let config::OsListItem::RemoteSubList(item) = self.boards.image(target) {
                let url = item.subitems_url.clone();
                tracing::debug!("Downloading subitems from {:?}", url);

                let target_clone: Vec<usize> = target.to_vec();
                let downloader = self.downloader.clone();

                return Task::perform(
                    async move { downloader.download_json_no_cache(url).await },
                    move |x| match x {
                        Ok(item) => BBImagerMessage::ResolveRemoteSubitemItem {
                            item,
                            target: target_clone.clone(),
                        },
                        Err(e) => {
                            tracing::warn!("Failed to download subitems with error {e}");
                            BBImagerMessage::Null
                        }
                    },
                );
            } else {
                return Task::none();
            }
        };

        let remote_image_jobs = os_images
            .clone()
            .into_iter()
            .filter_map(|(idx, x)| {
                if let config::OsListItem::RemoteSubList(item) = x {
                    tracing::debug!("Fetch: {:?} at {}", item.subitems_url, idx);
                    Some((idx, item.subitems_url.clone()))
                } else {
                    None
                }
            })
            .map(|(idx, url)| {
                let mut new_target: Vec<usize> = target.to_vec();
                new_target.push(idx);

                let downloader = self.downloader.clone();
                let url_clone = url.clone();
                Task::perform(
                    async move {
                        downloader
                            .download_json_no_cache::<Vec<config::OsListItem>, url::Url>(url_clone)
                            .await
                    },
                    move |x| match x {
                        Ok(item) => BBImagerMessage::ResolveRemoteSubitemItem {
                            item,
                            target: new_target.clone(),
                        },
                        Err(e) => {
                            tracing::warn!("Failed to download subitems {:?} with error {e}", url);
                            BBImagerMessage::Null
                        }
                    },
                )
            });

        Task::batch(remote_image_jobs)
    }

    const fn theme(&self) -> iced::Theme {
        iced::Theme::Light
    }

    fn config(&self) -> FlashingCustomization {
        let flasher = self.flasher().expect("Missing Flasher");
        FlashingCustomization::new(
            flasher,
            self.selected_image.as_ref().expect("Missing os image"),
            &self.app_config,
        )
    }

    fn start_flashing(
        &mut self,
        customization: Option<FlashingCustomization>,
    ) -> Task<BBImagerMessage> {
        let board = &self
            .boards
            .device(self.selected_board.expect("Missing board"));
        let docs_url = board.documentation.clone();

        let customization = customization.unwrap_or(self.config());
        let img = self.selected_image.clone();
        let dst = self.selected_dst.clone();

        tracing::info!("Starting Flashing Process");
        tracing::info!("Selected Board: {:#?}", board);
        tracing::info!("Selected Image: {:#?}", img);
        tracing::info!("Selected Destination: {:#?}", dst);
        tracing::info!("Selected Customization: {:#?}", customization);

        let s = iced::stream::channel(20, move |mut chan| async move {
            let _ = chan
                .send(BBImagerMessage::ProgressBar(ProgressBarState::PREPARING))
                .await;

            let (tx, mut rx) = iced::futures::channel::mpsc::channel(19);

            let flash_task =
                tokio::spawn(async move { helpers::flash(img, customization, dst, tx).await });
            let mut chan_clone = chan.clone();
            let progress_task = tokio::spawn(async move {
                while let Some(progress) = rx.next().await {
                    let _ = chan_clone.try_send(BBImagerMessage::ProgressBar(progress.into()));
                }
            });

            let res = flash_task
                .await
                .expect("Tokio runtime failed to spawn task");

            let res = match res {
                Ok(_) => {
                    tracing::info!("Flashing Successfull");
                    BBImagerMessage::StopFlashing(ProgressBarState::FLASHING_SUCCESS)
                }
                Err(e) => {
                    tracing::error!("Flashing failed with error: {e}");
                    BBImagerMessage::StopFlashing(ProgressBarState::fail(format!(
                        "Flashing Failed {e}"
                    )))
                }
            };

            let _ = chan.send(res).await;
            progress_task.abort();
        });

        let (t, h) = Task::stream(s).abortable();

        self.cancel_flashing = Some(h);

        Task::done(BBImagerMessage::SwitchScreen(Screen::Flashing(
            pages::FlashingState::new(
                ProgressBarState::PREPARING,
                docs_url.as_ref().map(|x| x.to_string()).unwrap_or_default(),
            ),
        )))
        .chain(t)
    }

    fn subscription(&self) -> Subscription<BBImagerMessage> {
        if let Some(flasher) = self.flasher() {
            // Do not use subscription for static destinations
            // Also do not use subscription when on other screens. Can cause Udisk2 to crash.
            if self.is_destionation_selectable()
                && self
                    .screen
                    .last()
                    .expect("No screen")
                    .is_destination_selection()
            {
                tracing::debug!("Refresh destinations for {:?}", flasher);
                let stream = iced::futures::stream::unfold(flasher, move |f| async move {
                    let mut dsts = helpers::destinations(flasher).await;
                    dsts.sort_by_key(|a| a.size());
                    let dsts = BBImagerMessage::Destinations(dsts);
                    Some((dsts, f))
                })
                .throttle(Duration::from_secs(1));

                return Subscription::run_with_id(flasher, stream);
            }
        }

        Subscription::none()
    }

    fn flasher(&self) -> Option<config::Flasher> {
        if let Some(x) = &self.selected_image {
            return Some(x.flasher());
        }
        let dev = self.boards.device(self.selected_board?);
        Some(dev.flasher)
    }

    pub(crate) fn selected_device(&self) -> Option<&config::Device> {
        self.selected_board.map(|x| self.boards.device(x))
    }

    pub(crate) fn selected_image(&self) -> Option<&helpers::BoardImage> {
        self.selected_image.as_ref()
    }

    pub(crate) fn selected_destination(&self) -> Option<&helpers::Destination> {
        self.selected_dst.as_ref()
    }

    pub(crate) fn is_destionation_selectable(&self) -> bool {
        if let Some(flasher) = self.flasher() {
            helpers::is_destination_selectable(flasher)
        } else {
            false
        }
    }

    pub(crate) fn devices(&self) -> impl Iterator<Item = (usize, &config::Device)> {
        self.boards.devices()
    }

    pub(crate) fn images(&self, idx: &[usize]) -> Option<Vec<(usize, &config::OsListItem)>> {
        self.boards.images(self.selected_board.unwrap(), idx)
    }

    pub(crate) fn destinations(&self) -> impl Iterator<Item = &helpers::Destination> {
        self.destinations.iter()
    }

    pub(crate) fn downloader(&self) -> &bb_downloader::Downloader {
        &self.downloader
    }

    pub(crate) fn customization(&self) -> Option<&FlashingCustomization> {
        self.customization.as_ref()
    }

    pub(crate) fn app_settings(&self) -> persistance::AppSettings {
        self.app_config.app_settings().cloned().unwrap_or_default()
    }

    pub(crate) fn timezones(&self) -> &widget::combo_box::State<String> {
        &self.timezones
    }

    pub(crate) fn keymaps(&self) -> &widget::combo_box::State<String> {
        &self.keymaps
    }

    pub(crate) fn is_flashing(&self) -> bool {
        self.cancel_flashing.is_some()
    }
}
