#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{borrow::Cow, collections::HashSet, time::Duration};

use helpers::{ProgressBarState, Tainted};
use iced::{futures::SinkExt, widget, Element, Subscription, Task};
use pages::{configuration::FlashingCustomization, Screen};
use tokio_stream::StreamExt;
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
        .subscription(BBImager::subscription)
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
    selected_image: Option<helpers::BoardImage>,
    selected_dst: Option<bb_imager::Destination>,
    destinations: HashSet<bb_imager::Destination>,
    search_bar: String,
    cancel_flashing: Option<iced::task::Handle>,
    customization: Option<Tainted<FlashingCustomization>>,
    flashing_state: Option<pages::flash::FlashingState>,

    timezones: widget::combo_box::State<String>,
    keymaps: widget::combo_box::State<String>,

    // Flag to indicate if destinations are selectable
    destination_selectable: bool,
}

#[derive(Debug, Clone)]
enum BBImagerMessage {
    UpdateConfig(helpers::Boards),
    BoardSelected(String),
    SelectImage(helpers::BoardImage),
    SelectLocalImage,
    SelectPort(bb_imager::Destination),
    ProgressBar(ProgressBarState),
    SwitchScreen(Screen),
    Search(String),
    Destinations(HashSet<bb_imager::Destination>),
    Reset,
    ResetConfig,

    Flash,
    StartFlashing,
    StartFlashingWithoutConfiguraton,
    CancelFlashing,
    StopFlashing(ProgressBarState),
    UpdateFlashConfig(FlashingCustomization),

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
            destination_selectable: true,
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
                self.customization.take();

                let icons: HashSet<url::Url> =
                    self.boards.images(&x).map(|x| x.icon.clone()).collect();

                self.selected_board = Some(x);
                self.back_home();

                let refresh_job = self.refresh_destinations();

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

                return Task::batch(jobs.chain([refresh_job]));
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
                        Some(y) => BBImagerMessage::SelectImage(helpers::BoardImage::Image(
                            bb_imager::SelectedImage::local(y),
                        )),
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
            BBImagerMessage::ResetConfig => {
                self.customization = Some(Tainted::new(self.config()));
            }
            BBImagerMessage::SwitchScreen(x) => {
                self.screen = x;
                match x {
                    Screen::Home => self.back_home(),
                    Screen::ExtraConfiguration => {
                        if self.customization.is_none() {
                            self.customization = Some(Tainted::new(self.config()))
                        }
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
            BBImagerMessage::Flash => {
                self.screen = Screen::FlashingConfirmation;
            }
            BBImagerMessage::StartFlashing => {
                return self
                    .start_flashing(Some(self.customization.as_ref().unwrap().as_ref().clone()));
            }
            BBImagerMessage::StartFlashingWithoutConfiguraton => {
                return self.start_flashing(None);
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
                if !self.destination_selectable {
                    assert_eq!(x.len(), 1);
                    let temp: Vec<&bb_imager::Destination> = x.iter().collect();
                    self.selected_dst = Some(temp[0].clone());
                }
                self.destinations = x;
            }
            BBImagerMessage::UpdateFlashConfig(x) => {
                self.customization = Some(Tainted::new_tainted(x))
            }
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
                self.destination_selectable,
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
                            BBImagerMessage::SelectImage(helpers::BoardImage::SdFormat),
                        ),
                    ]
                    .into_iter(),
                )
            }
            Screen::DestinationSelection => {
                pages::destination_selection::view(self.destinations.iter(), &self.search_bar)
            }
            Screen::ExtraConfiguration => pages::configuration::view(
                self.customization.as_ref().unwrap().as_ref(),
                &self.timezones,
                &self.keymaps,
            ),
            Screen::Flashing => pages::flash::view(
                self.flashing_state.as_ref().unwrap(),
                self.cancel_flashing.is_some(),
            ),
            Screen::FlashingConfirmation => {
                let base = pages::home::view(
                    self.selected_board.as_deref(),
                    self.selected_image.as_ref(),
                    self.selected_dst.as_ref(),
                    self.destination_selectable,
                );
                let menu = widget::column![
                    widget::text("Would you like to apply customization settings?"),
                    widget::row![
                        widget::button("Edit Settings")
                            .on_press(BBImagerMessage::SwitchScreen(Screen::ExtraConfiguration)),
                        widget::button("Yes").on_press_maybe(
                            if self.customization.as_ref().map(|x| x.is_tainted()) == Some(true) {
                                Some(BBImagerMessage::StartFlashing)
                            } else {
                                None
                            }
                        ),
                        widget::button("No")
                            .on_press(BBImagerMessage::StartFlashingWithoutConfiguraton),
                        widget::button("Abort")
                            .on_press(BBImagerMessage::SwitchScreen(Screen::Home))
                    ]
                    .spacing(8)
                ]
                .align_x(iced::Alignment::Center)
                .padding(16)
                .spacing(16);

                let menu = widget::container(menu)
                    .style(|_| widget::container::background(iced::Color::WHITE));

                let overlay = widget::opaque(widget::center(menu).style(|_| {
                    widget::container::background(iced::Color {
                        a: 0.8,
                        ..iced::Color::BLACK
                    })
                }));
                widget::stack![base, overlay].into()
            }
        }
    }

    const fn theme(&self) -> iced::Theme {
        iced::Theme::Light
    }

    fn back_home(&mut self) {
        self.search_bar.clear();
        self.screen = Screen::Home;
    }

    fn refresh_destinations(&mut self) -> Task<BBImagerMessage> {
        let flasher = self
            .boards
            .device(self.selected_board.as_ref().expect("Missing board"))
            .flasher;

        self.destination_selectable = flasher.destination_selectable();

        // Do not use subscription for static destinations
        if !self.destination_selectable {
            Task::perform(
                async move { flasher.destinations().await },
                BBImagerMessage::Destinations,
            )
        } else {
            Task::none()
        }
    }

    fn config(&self) -> FlashingCustomization {
        let flasher = self
            .boards
            .device(self.selected_board.as_ref().expect("Missing board"))
            .flasher;
        FlashingCustomization::new(
            flasher,
            self.selected_image.as_ref().expect("Missing os image"),
        )
    }

    fn flashing_config(
        &self,
        customization: FlashingCustomization,
    ) -> bb_imager::common::FlashingConfig {
        match (
            self.selected_image.clone(),
            customization,
            self.selected_dst.clone(),
        ) {
            (
                Some(helpers::BoardImage::SdFormat),
                FlashingCustomization::LinuxSdFormat,
                Some(bb_imager::Destination::SdCard { path, .. }),
            ) => bb_imager::common::FlashingConfig::LinuxSdFormat { dst: path },
            (
                Some(helpers::BoardImage::Image(img)),
                FlashingCustomization::LinuxSd(customization),
                Some(bb_imager::Destination::SdCard { path, .. }),
            ) => bb_imager::common::FlashingConfig::LinuxSd {
                img,
                dst: path,
                customization,
            },
            (
                Some(helpers::BoardImage::Image(img)),
                FlashingCustomization::Bcf(customization),
                Some(bb_imager::Destination::Port(port)),
            ) => bb_imager::common::FlashingConfig::BeagleConnectFreedom {
                img,
                port,
                customization,
            },
            (
                Some(helpers::BoardImage::Image(img)),
                FlashingCustomization::Msp430,
                Some(bb_imager::Destination::HidRaw(port)),
            ) => bb_imager::common::FlashingConfig::Msp430 { img, port },
            #[cfg(feature = "pb2_mspm0")]
            (
                Some(helpers::BoardImage::Image(img)),
                FlashingCustomization::Pb2Mspm0 { persist_eeprom },
                _,
            ) => bb_imager::common::FlashingConfig::Pb2Mspm0 {
                img,
                persist_eeprom,
            },
            _ => unreachable!(),
        }
    }

    fn start_flashing(
        &mut self,
        customization: Option<FlashingCustomization>,
    ) -> Task<BBImagerMessage> {
        let docs_url = &self
            .boards
            .device(self.selected_board.as_ref().expect("Missing board"))
            .documentation;
        self.screen = Screen::Flashing;
        self.flashing_state = Some(pages::flash::FlashingState::new(
            ProgressBarState::PREPARING,
            docs_url.to_string(),
        ));

        let downloader = self.downloader.clone();

        let config = self.flashing_config(customization.unwrap_or(self.config()));

        let s = iced::stream::channel(20, move |mut chan| async move {
            let _ = chan
                .send(BBImagerMessage::ProgressBar(ProgressBarState::PREPARING))
                .await;

            let (tx, mut rx) = tokio::sync::mpsc::channel(20);

            let flash_task = tokio::spawn(config.download_flash_customize(downloader, tx));

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

        t
    }

    fn subscription(&self) -> Subscription<BBImagerMessage> {
        if let Some(board) = self.selected_board.as_ref() {
            // Do not use subscription for static destinations
            // Also do not use subscription when on other screens. Can cause Udisk2 to crash.
            if self.destination_selectable && self.screen == Screen::DestinationSelection {
                let flasher = self.boards.device(board).flasher;

                let stream = futures_util::stream::unfold(flasher, |f| async move {
                    let dsts = f.destinations().await;
                    let dsts = BBImagerMessage::Destinations(dsts);
                    Some((dsts, f))
                })
                .throttle(Duration::from_secs(1));

                return Subscription::run_with_id(board.to_string(), stream);
            }
        }

        Subscription::none()
    }
}
