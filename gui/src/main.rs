#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{borrow::Cow, collections::HashSet};

use bb_imager::FlashingConfig;
use helpers::ProgressBarState;
use iced::{futures::SinkExt, widget, Element, Task};
use pages::Screen;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod constants;
mod helpers;
mod pages;

fn main() -> iced::Result {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .expect("Failed to register tracing_subscriber");

    let icon = iced::window::icon::from_file_data(
        constants::WINDOW_ICON,
        Some(iced::advanced::graphics::image::image_rs::ImageFormat::Png),
    )
    .ok();
    assert!(icon.is_some());

    #[cfg(target_os = "macos")]
    // HACK: mac_notification_sys set application name (not an option in notify-rust)
    let _ = notify_rust::set_application("org.beagleboard.imagingutility");

    let config = bb_imager::config::Config::from_json(constants::DEFAULT_CONFIG)
        .expect("Failed to parse config");
    let boards = helpers::Boards::from(config);

    let settings = iced::window::Settings {
        min_size: Some(constants::WINDOW_SIZE),
        size: constants::WINDOW_SIZE,
        ..Default::default()
    };

    iced::application(constants::APP_NAME, BBImager::update, BBImager::view)
        .theme(BBImager::theme)
        .window(settings)
        .font(constants::FONT_REGULAR_BYTES)
        .font(constants::FONT_BOLD_BYTES)
        .default_font(constants::FONT_REGULAR)
        .run_with(move || BBImager::new(boards))
}

#[derive(Default, Debug)]
struct BBImager {
    boards: helpers::Boards,
    downloader: bb_imager::download::Downloader,
    screen: Screen,
    selected_board: Option<String>,
    selected_image: Option<bb_imager::common::SelectedImage>,
    selected_dst: Option<bb_imager::Destination>,
    destinations: HashSet<bb_imager::Destination>,
    search_bar: String,
    cancel_flashing: Option<iced::task::Handle>,
    flashing_config: Option<bb_imager::FlashingConfig>,
    flashing_state: Option<pages::flash::FlashingState>,

    timezones: widget::combo_box::State<String>,
    keymaps: widget::combo_box::State<String>,
}

#[derive(Debug, Clone)]
enum BBImagerMessage {
    UpdateConfig(helpers::Boards),
    BoardSelected(String),
    SelectImage(bb_imager::SelectedImage),
    SelectLocalImage,
    SelectPort(bb_imager::Destination),
    ProgressBar(ProgressBarState),
    SwitchScreen(Screen),
    Search(String),
    Destinations(HashSet<bb_imager::Destination>),
    RefreshDestinations,
    Reset,

    StartFlashing,
    CancelFlashing,
    StopFlashing(ProgressBarState),
    UpdateFlashConfig(bb_imager::FlashingConfig),

    OpenUrl(Cow<'static, str>),

    Null,
}

impl BBImager {
    fn new(boards: helpers::Boards) -> (Self, Task<BBImagerMessage>) {
        let downloader = bb_imager::download::Downloader::default();

        // Fetch old config
        let client = downloader.clone();
        let boards_clone = boards.clone();
        let config_task = Task::perform(
            async move {
                let data: bb_imager::config::compact::Config = client
                    .download_json(constants::BB_IMAGER_ORIGINAL_CONFIG)
                    .await
                    .map_err(|e| format!("Config parsing failed: {e}"))?;

                // If spawn_blocking fails, there is a problem with the underlying runtime
                tokio::task::spawn_blocking(|| Ok(boards_clone.merge(data.into())))
                    .await
                    .expect("Tokio runtime failed to spawn blocking task")
            },
            |x: Result<helpers::Boards, String>| match x {
                Ok(y) => BBImagerMessage::UpdateConfig(y),
                Err(e) => {
                    tracing::error!("Failed to fetch config: {e}");
                    BBImagerMessage::Null
                }
            },
        );

        let ans = Self {
            boards,
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
            ..Default::default()
        };

        // Fetch all board images
        let board_image_task = ans.fetch_board_images();

        (ans, Task::batch([config_task, board_image_task]))
    }

    fn fetch_board_images(&self) -> Task<BBImagerMessage> {
        // Do not try downloading same image multiple times
        let icons: HashSet<url::Url> = self
            .boards
            .devices()
            .map(|(_, dev)| dev.icon.clone())
            .collect();

        let tasks = icons.into_iter().map(|icon| {
            Task::perform(
                self.downloader.clone().download_without_sha(icon.clone()),
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

    fn update(&mut self, message: BBImagerMessage) -> Task<BBImagerMessage> {
        match message {
            BBImagerMessage::UpdateConfig(c) => {
                self.boards = c;
                return self.fetch_board_images();
            }
            BBImagerMessage::BoardSelected(x) => {
                // Reset any previously selected values
                self.selected_dst.take();
                self.selected_image.take();
                self.destinations.clear();

                let icons: HashSet<url::Url> =
                    self.boards.images(&x).map(|x| x.icon.clone()).collect();

                self.selected_board = Some(x);
                self.back_home();

                let jobs = icons.into_iter().map(|x| {
                    Task::perform(
                        self.downloader.clone().download_without_sha(x.clone()),
                        move |p| match p {
                            Ok(_path) => BBImagerMessage::Null,
                            Err(e) => {
                                tracing::warn!("Failed to download image {x} with error {e}");
                                BBImagerMessage::Null
                            }
                        },
                    )
                });

                return Task::batch(jobs.chain([self.refresh_destinations()]));
            }
            BBImagerMessage::ProgressBar(x) => {
                if let Some(state) = self.flashing_state.take() {
                    self.flashing_state = Some(state.update(x));
                }
            }
            BBImagerMessage::SelectImage(x) => {
                self.selected_image = Some(x);
                self.back_home();
            }
            BBImagerMessage::SelectLocalImage => {
                let flasher = self
                    .boards
                    .device(
                        self.selected_board
                            .as_ref()
                            .expect("Image cannot be selected before board"),
                    )
                    .flasher;
                let (name, extensions) = flasher.file_filter();
                return Task::perform(
                    async move {
                        rfd::AsyncFileDialog::new()
                            .add_filter(name, extensions)
                            .pick_file()
                            .await
                            .map(|x| x.path().to_path_buf())
                    },
                    |x| match x {
                        Some(y) => BBImagerMessage::SelectImage(bb_imager::SelectedImage::local(y)),
                        None => BBImagerMessage::Null,
                    },
                );
            }
            BBImagerMessage::SelectPort(x) => {
                self.selected_dst = Some(x);
                self.back_home();
            }
            BBImagerMessage::Reset => {
                self.selected_dst.take();
                self.selected_image.take();
                self.selected_board.take();
                self.search_bar.clear();
                self.destinations.clear();
            }
            BBImagerMessage::SwitchScreen(x) => {
                self.screen = x;
                match x {
                    Screen::Home => self.back_home(),
                    Screen::DestinationSelection => {
                        return self.refresh_destinations();
                    }
                    Screen::ExtraConfiguration => {
                        let flasher = self
                            .boards
                            .device(self.selected_board.as_ref().expect("Missing board"))
                            .flasher;
                        self.flashing_config = Some(FlashingConfig::new(
                            flasher,
                            self.selected_image.as_ref().expect("Missing os image"),
                        ));
                    }
                    _ => {}
                }
            }
            BBImagerMessage::Search(x) => {
                self.search_bar = x;
            }
            BBImagerMessage::CancelFlashing => {
                if let Some(task) = self.cancel_flashing.take() {
                    task.abort();
                }

                if let Some(x) = &self.flashing_state {
                    if let Some(y) = x.progress.cancel() {
                        return Task::done(BBImagerMessage::StopFlashing(y));
                    }
                }
            }
            BBImagerMessage::StartFlashing => {
                let docs_url = &self
                    .boards
                    .device(self.selected_board.as_ref().expect("Missing board"))
                    .documentation;
                self.screen = Screen::Flashing;
                self.flashing_state = Some(pages::flash::FlashingState::new(
                    ProgressBarState::PREPARING,
                    docs_url.to_string(),
                ));

                let dst = self.selected_dst.clone().expect("No destination selected");
                let img = self.selected_image.clone().expect("Missing os image");
                let downloader = self.downloader.clone();
                let config = self
                    .flashing_config
                    .clone()
                    .expect("Missing flashing config");

                let s = iced::stream::channel(20, move |mut chan| async move {
                    let _ = chan
                        .send(BBImagerMessage::ProgressBar(ProgressBarState::PREPARING))
                        .await;

                    let (tx, mut rx) = tokio::sync::mpsc::channel(20);

                    let flash_task = tokio::spawn(
                        bb_imager::common::Flasher::new(img, dst, downloader, tx, config)
                            .download_flash_customize(),
                    );

                    while let Some(progress) = rx.recv().await {
                        let _ = chan.try_send(BBImagerMessage::ProgressBar(progress.into()));
                    }

                    let res = flash_task
                        .await
                        .expect("Tokio runtime failed to spawn task");
                    let res = match res {
                        Ok(_) => BBImagerMessage::StopFlashing(ProgressBarState::FLASHING_SUCCESS),
                        Err(e) => BBImagerMessage::StopFlashing(ProgressBarState::fail(format!(
                            "Flashing Failed {e}"
                        ))),
                    };

                    let _ = chan.send(res).await;
                });

                let (t, h) = Task::stream(s).abortable();

                self.cancel_flashing = Some(h);

                return t;
            }
            BBImagerMessage::StopFlashing(x) => {
                let _ = self.cancel_flashing.take();
                let content = x.content();

                let progress_task = Task::done(BBImagerMessage::ProgressBar(x));
                let notification_task = Task::future(async move {
                    let res = tokio::task::spawn_blocking(move || {
                        notify_rust::Notification::new()
                            .appname("BeagleBoard Imager")
                            .body(&content)
                            .finalize()
                            .show()
                    })
                    .await
                    .expect("Tokio runtime failed to spawn blocking task");

                    tracing::debug!("Notification response {res:?}");
                    BBImagerMessage::Null
                });

                return Task::batch([progress_task, notification_task]);
            }
            BBImagerMessage::Destinations(x) => {
                self.destinations = x;
            }
            BBImagerMessage::RefreshDestinations => {
                return self.refresh_destinations();
            }
            BBImagerMessage::UpdateFlashConfig(x) => self.flashing_config = Some(x),
            BBImagerMessage::OpenUrl(x) => {
                return Task::future(async move {
                    let res = webbrowser::open(&x);
                    tracing::info!("Open Url Resp {res:?}");
                    BBImagerMessage::Null
                });
            }
            BBImagerMessage::Null => {}
        };

        Task::none()
    }

    fn view(&self) -> Element<BBImagerMessage> {
        match &self.screen {
            Screen::Home => pages::home::view(
                self.selected_board.as_deref(),
                self.selected_image.as_ref(),
                self.selected_dst.as_ref(),
            ),
            Screen::BoardSelection => {
                pages::board_selection::view(&self.boards, &self.search_bar, &self.downloader)
            }
            Screen::ImageSelection => {
                let board = self.selected_board.as_ref().expect("Missing Board");
                let images = self.boards.images(board);
                pages::image_selection::view(
                    images,
                    &self.search_bar,
                    &self.downloader,
                    [
                        pages::image_selection::ExtraImageEntry::new(
                            "Custom Image",
                            constants::FILE_ADD_ICON,
                            BBImagerMessage::SelectLocalImage,
                        ),
                        pages::image_selection::ExtraImageEntry::new(
                            "Format Sd Card",
                            constants::FORMAT_ICON,
                            BBImagerMessage::SelectImage(bb_imager::SelectedImage::Null(
                                "Format Sd Card",
                            )),
                        ),
                    ]
                    .into_iter(),
                )
            }
            Screen::DestinationSelection => {
                pages::destination_selection::view(self.destinations.iter(), &self.search_bar)
            }
            Screen::ExtraConfiguration => pages::configuration::view(
                self.flashing_config.as_ref(),
                &self.timezones,
                &self.keymaps,
            ),
            Screen::Flashing => pages::flash::view(
                self.flashing_state.as_ref().unwrap(),
                self.cancel_flashing.is_some(),
            ),
        }
    }

    const fn theme(&self) -> iced::Theme {
        iced::Theme::Light
    }

    fn back_home(&mut self) {
        self.search_bar.clear();
        self.screen = Screen::Home;
    }

    fn refresh_destinations(&self) -> Task<BBImagerMessage> {
        let flasher = self
            .boards
            .device(self.selected_board.as_ref().expect("Missing board"))
            .flasher;

        Task::perform(
            async move { flasher.destinations().await },
            BBImagerMessage::Destinations,
        )
    }
}
