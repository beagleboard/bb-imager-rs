#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{collections::HashSet, time::Duration};

use constants::PACKAGE_QUALIFIER;
use helpers::ProgressBarState;
use iced::{Element, Subscription, Task, futures::SinkExt, widget};
use message::BBImagerMessage;
use pages::{Screen, configuration::FlashingCustomization};
use tokio_stream::StreamExt;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod constants;
mod helpers;
mod message;
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

    let app_config = helpers::GuiConfiguration::load().unwrap_or_default();

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
        .run_with(move || BBImager::new(boards, app_config))
}

#[derive(Debug)]
struct BBImager {
    app_config: helpers::GuiConfiguration,
    boards: helpers::Boards,
    downloader: bb_downloader::Downloader,
    screen: Vec<Screen>,
    selected_board: Option<usize>,
    selected_image: Option<helpers::BoardImage>,
    selected_dst: Option<bb_imager::Destination>,
    destinations: HashSet<bb_imager::Destination>,
    search_bar: String,
    cancel_flashing: Option<iced::task::Handle>,
    customization: Option<FlashingCustomization>,
    flashing_state: Option<pages::flash::FlashingState>,

    timezones: widget::combo_box::State<String>,
    keymaps: widget::combo_box::State<String>,

    // Flag to indicate if destinations are selectable
    destination_selectable: bool,
}

impl BBImager {
    fn new(
        boards: helpers::Boards,
        app_config: helpers::GuiConfiguration,
    ) -> (Self, Task<BBImagerMessage>) {
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
        let boards_clone = boards.clone();
        let config_task = Task::perform(
            async move {
                let data: bb_imager::config::Config = client
                    .download_json_no_cache(constants::BB_IMAGER_ORIGINAL_CONFIG)
                    .await
                    .map_err(|e| format!("Config parsing failed: {e}"))?;

                // If spawn_blocking fails, there is a problem with the underlying runtime
                tokio::task::spawn_blocking(|| Ok(boards_clone.merge(data)))
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

        let mut ans = Self {
            app_config,
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
            screen: Vec::with_capacity(3),
            selected_board: Default::default(),
            selected_image: Default::default(),
            selected_dst: Default::default(),
            destinations: Default::default(),
            search_bar: Default::default(),
            cancel_flashing: Default::default(),
            customization: Default::default(),
            flashing_state: Default::default(),
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
                async move { downloader.download(icon_clone, None).await },
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
                tracing::info!("Config: {:#?}", c);
                self.boards = c;
                return self.fetch_board_images();
            }
            BBImagerMessage::ResolveRemoteSubitemItem { item, target } => {
                self.boards.resolve_remote_subitem(item, &target);
            }
            BBImagerMessage::BoardSelected(x) => {
                // Reset any previously selected values
                self.selected_dst.take();
                self.selected_image.take();
                self.destinations.clear();
                self.customization.take();

                let os_images = self
                    .boards
                    .images(x, &[])
                    .expect("Initial image list can never be None");

                let remote_image_jobs = self.fetch_remote_subitems(x, &[]);
                let icons: HashSet<url::Url> = os_images.iter().map(|(_, x)| x.icon()).collect();
                self.selected_board = Some(x);

                let jobs = icons.into_iter().map(|x| {
                    let downloader = self.downloader.clone();
                    let x_clone = x.clone();
                    Task::perform(
                        async move { downloader.download(x_clone, None).await },
                        move |p| match p {
                            Ok(_path) => BBImagerMessage::Null,
                            Err(e) => {
                                tracing::warn!("Failed to download image {x} with error {e}");
                                BBImagerMessage::Null
                            }
                        },
                    )
                });

                // Close Board selection page
                self.screen.pop();

                return Task::batch(jobs.chain([remote_image_jobs]));
            }
            BBImagerMessage::ProgressBar(x) => {
                if let Some(state) = self.flashing_state.take() {
                    self.flashing_state = Some(state.update(x));
                }
            }
            BBImagerMessage::SelectImage(x) => {
                tracing::info!("Selected Image: {}", x);
                self.selected_image = Some(x);
                self.screen.clear();
                self.screen.push(Screen::Home);

                return self.refresh_destinations();
            }
            BBImagerMessage::SelectLocalImage(flasher) => {
                let (name, extensions) = flasher.file_filter();
                return Task::perform(
                    async move {
                        rfd::AsyncFileDialog::new()
                            .add_filter(name, extensions)
                            .pick_file()
                            .await
                            .map(|x| x.path().to_path_buf())
                    },
                    move |x| match x {
                        Some(y) => {
                            BBImagerMessage::SelectImage(helpers::BoardImage::local(y, flasher))
                        }
                        None => BBImagerMessage::Null,
                    },
                );
            }
            BBImagerMessage::SelectPort(x) => {
                self.selected_dst = Some(x);
                self.screen.pop();
            }
            BBImagerMessage::Reset => {
                self.selected_dst.take();
                self.selected_image.take();
                self.selected_board.take();
                self.search_bar.clear();
                self.destinations.clear();
            }
            BBImagerMessage::SwitchScreen(x) => {
                self.screen.clear();
                return self.push_page(x);
            }
            BBImagerMessage::ReplaceScreen(x) => {
                self.screen.pop();
                return self.push_page(x);
            }
            BBImagerMessage::PushScreen(x) => {
                tracing::debug!("Push Page: {:?}", x);
                return self.push_page(x);
            }
            BBImagerMessage::PopScreen => {
                tracing::debug!("Pop screen");
                self.screen.pop();
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
                return self.start_flashing(self.customization.clone());
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
            BBImagerMessage::UpdateFlashConfig(x) => self.customization = Some(x),
            BBImagerMessage::OpenUrl(x) => {
                return Task::future(async move {
                    let res = webbrowser::open(&x);
                    tracing::info!("Open Url Resp {res:?}");
                    BBImagerMessage::Null
                });
            }
            BBImagerMessage::SaveCustomization => {
                match self.customization.clone().unwrap() {
                    FlashingCustomization::LinuxSd(c) => self.app_config.update_sd_customization(c),
                    FlashingCustomization::Bcf(c) => self.app_config.update_bcf_customization(c),
                    _ => {}
                }

                let config = self.app_config.clone();

                // Since we have a cache of config, no need to wait for disk persistance.
                self.screen.pop();

                return Task::future(async move {
                    if let Err(e) = config.save().await {
                        tracing::error!("Failed to save config: {e}");
                    }
                    BBImagerMessage::Null
                });
            }
            BBImagerMessage::ResetCustomization => {
                self.customization = Some(self.customization.clone().unwrap().reset());
            }
            BBImagerMessage::CancelCustomization => {
                self.screen.pop();
                self.customization = Some(self.config());
            }
            BBImagerMessage::Null => {}
        };

        Task::none()
    }

    fn push_page(&mut self, x: Screen) -> Task<BBImagerMessage> {
        self.search_bar.clear();
        self.screen.push(x.clone());

        match x {
            Screen::ExtraConfiguration if self.customization.is_none() => {
                self.customization = Some(self.config())
            }
            Screen::ImageSelection(page) => {
                tracing::info!("Image Selection Screen");
                let board = self.selected_board.unwrap();
                return self.fetch_remote_subitems(board, &page.idx);
            }
            _ => {}
        }

        Task::none()
    }

    fn fetch_remote_subitems(&self, board: usize, target: &[usize]) -> Task<BBImagerMessage> {
        let Some(os_images) = self.boards.images(board, target) else {
            // Maybe resolving was missed
            if let bb_imager::config::OsListItem::RemoteSubList { subitems_url, .. } =
                self.boards.image(target)
            {
                let url = subitems_url.clone();
                tracing::info!("Downloading subites from {:?}", url);

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
                if let bb_imager::config::OsListItem::RemoteSubList { subitems_url, .. } = x {
                    tracing::info!("Fetch: {:?} at {}", subitems_url, idx);
                    Some((idx, subitems_url.clone()))
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
                            .download_json_no_cache::<Vec<bb_imager::config::OsListItem>, url::Url>(
                                url_clone,
                            )
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

    fn view(&self) -> Element<BBImagerMessage> {
        tracing::debug!("Page Stack: {:#?}", self.screen);

        match self.screen.last().expect("No Screen") {
            Screen::Home => pages::home::view(
                self.selected_board.map(|x| self.boards.device(x)),
                self.selected_image.as_ref(),
                self.selected_dst.as_ref(),
                self.destination_selectable,
            ),
            Screen::BoardSelection => {
                pages::board_selection::view(&self.boards, &self.search_bar, &self.downloader)
            }
            Screen::ImageSelection(page) if page.flasher == bb_imager::Flasher::SdCard => {
                let board = self.selected_board.expect("Missing Board");
                let flasher = page.flasher;

                page.view(
                    self.boards.images(board, &page.idx),
                    &self.search_bar,
                    &self.downloader,
                    [
                        pages::image_selection::ExtraImageEntry::new(
                            "Custom Image",
                            constants::FILE_ADD_ICON,
                            BBImagerMessage::SelectLocalImage(flasher),
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
            Screen::ImageSelection(page) => {
                let board = self.selected_board.expect("Missing Board");
                let flasher = page.flasher;

                page.view(
                    self.boards.images(board, &page.idx),
                    &self.search_bar,
                    &self.downloader,
                    [pages::image_selection::ExtraImageEntry::new(
                        "Custom Image",
                        constants::FILE_ADD_ICON,
                        BBImagerMessage::SelectLocalImage(flasher),
                    )]
                    .into_iter(),
                )
            }
            Screen::DestinationSelection => {
                pages::destination_selection::view(self.destinations.iter(), &self.search_bar)
            }
            Screen::ExtraConfiguration => pages::configuration::view(
                self.customization.as_ref().unwrap(),
                &self.timezones,
                &self.keymaps,
            ),
            Screen::Flashing => pages::flash::view(
                self.flashing_state.as_ref().unwrap(),
                self.cancel_flashing.is_some(),
            ),
            Screen::FlashingConfirmation => {
                let base = pages::home::view(
                    self.selected_board.map(|x| self.boards.device(x)),
                    self.selected_image.as_ref(),
                    self.selected_dst.as_ref(),
                    self.destination_selectable,
                );
                let menu = widget::column![
                    widget::text("Would you like to apply customization settings?"),
                    widget::row![
                        widget::button("Edit Settings")
                            .on_press(BBImagerMessage::ReplaceScreen(Screen::ExtraConfiguration)),
                        widget::button("Yes").on_press(BBImagerMessage::StartFlashing),
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

    fn refresh_destinations(&mut self) -> Task<BBImagerMessage> {
        if let Some(flasher) = self.flasher() {
            self.destination_selectable = flasher.destination_selectable();

            // Do not use subscription for static destinations
            if !self.destination_selectable {
                return Task::perform(
                    async move { flasher.destinations().await },
                    BBImagerMessage::Destinations,
                );
            }
        }

        Task::none()
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
        let docs_url = &self
            .boards
            .device(self.selected_board.expect("Missing board"))
            .documentation;
        self.flashing_state = Some(pages::flash::FlashingState::new(
            ProgressBarState::PREPARING,
            docs_url.as_ref().map(|x| x.to_string()).unwrap_or_default(),
        ));

        let customization = customization.unwrap_or(self.config());
        let img = self.selected_image.clone();
        let dst = self.selected_dst.clone();

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
                Ok(_) => BBImagerMessage::StopFlashing(ProgressBarState::FLASHING_SUCCESS),
                Err(e) => BBImagerMessage::StopFlashing(ProgressBarState::fail(format!(
                    "Flashing Failed {e}"
                ))),
            };

            let _ = chan.send(res).await;
            progress_task.abort();
        });

        let (t, h) = Task::stream(s).abortable();

        self.cancel_flashing = Some(h);

        Task::done(BBImagerMessage::SwitchScreen(Screen::Flashing)).chain(t)
    }

    fn subscription(&self) -> Subscription<BBImagerMessage> {
        if let Some(flasher) = self.flasher() {
            // Do not use subscription for static destinations
            // Also do not use subscription when on other screens. Can cause Udisk2 to crash.
            if self.destination_selectable
                && self.screen.last().expect("No screen") == &Screen::DestinationSelection
            {
                let stream = iced::futures::stream::unfold(flasher, |f| async move {
                    let dsts = f.destinations().await;
                    let dsts = BBImagerMessage::Destinations(dsts);
                    Some((dsts, f))
                })
                .throttle(Duration::from_secs(1));

                return Subscription::run_with_id(flasher, stream);
            }
        }

        Subscription::none()
    }

    fn flasher(&self) -> Option<bb_imager::Flasher> {
        if let Some(x) = &self.selected_image {
            return Some(x.flasher());
        }
        let dev = self.boards.device(self.selected_board?);
        Some(dev.flasher)
    }
}
