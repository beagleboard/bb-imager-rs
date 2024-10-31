#![windows_subsystem = "windows"]

use std::{borrow::Cow, collections::HashSet};

use helpers::ProgressBarState;
use iced::{
    futures::SinkExt,
    widget::{self, button, text},
    Element, Task,
};

mod constants;
mod helpers;

fn main() -> iced::Result {
    tracing_subscriber::fmt().init();

    let icon = iced::window::icon::from_file_data(constants::WINDOW_ICON, None).ok();
    assert!(icon.is_some());

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

    timezones: Option<widget::combo_box::State<String>>,
    keymaps: Option<widget::combo_box::State<String>>,
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
    fn new(config: helpers::Boards) -> (Self, Task<BBImagerMessage>) {
        let downloader = bb_imager::download::Downloader::default();

        // Fetch old config
        let client = downloader.client();
        let boards_clone = config.clone();
        let config_task = Task::perform(
            async move {
                let data: bb_imager::config::compact::Config = client
                    .get(constants::BB_IMAGER_ORIGINAL_CONFIG)
                    .send()
                    .await
                    .map_err(|e| format!("Config download failed: {e}"))?
                    .json()
                    .await
                    .map_err(|e| format!("Config parsing failed: {e}"))?;
                tokio::task::spawn_blocking(|| Ok(boards_clone.merge(data.into())))
                    .await
                    .unwrap()
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
            boards: config,
            downloader: downloader.clone(),
            timezones: Some(timezone()),
            keymaps: Some(keymap()),
            ..Default::default()
        };

        // Fetch all board images
        let board_image_task = ans.fetch_board_images();

        (ans, Task::batch([config_task, board_image_task]))
    }

    fn fetch_board_images(&self) -> Task<BBImagerMessage> {
        let icons: HashSet<url::Url> = self
            .boards
            .devices()
            .map(|(_, dev)| dev.icon.clone())
            .collect();

        let tasks = icons.into_iter().map(|icon| {
            Task::perform(
                self.downloader.clone().download_image(icon.clone()),
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
                        self.downloader.clone().download_image(x.clone()),
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
                if let Screen::Flashing(mut s) = self.screen.clone() {
                    s.progress = x;
                    s.running = self.cancel_flashing.is_some();
                    self.screen = Screen::Flashing(s)
                } else {
                    unreachable!()
                }
            }
            BBImagerMessage::SelectImage(x) => {
                self.selected_image = Some(x);
                self.back_home();
            }
            BBImagerMessage::SelectLocalImage => {
                let flasher = self
                    .boards
                    .device(self.selected_board.as_ref().unwrap())
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
                self.screen = x.clone();
                match x {
                    Screen::Home => self.back_home(),
                    Screen::DestinationSelection => {
                        return self.refresh_destinations();
                    }
                    Screen::ExtraConfiguration => {
                        let flasher = self
                            .boards
                            .device(self.selected_board.as_ref().unwrap())
                            .flasher;
                        self.flashing_config = Some(flasher.into());
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

                return Task::done(BBImagerMessage::StopFlashing(ProgressBarState::fail(
                    "Flashing Cancelled by user",
                )));
            }
            BBImagerMessage::StartFlashing => {
                let docs_url = &self
                    .boards
                    .device(self.selected_board.as_ref().unwrap())
                    .documentation;
                self.screen = Screen::Flashing(FlashingScreen::new(docs_url.to_string()));

                let dst = self.selected_dst.clone().expect("No destination selected");
                let img = self.selected_image.clone().unwrap();
                let downloader = self.downloader.clone();
                let config = self.flashing_config.clone().unwrap();

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

                    let res = flash_task.await.unwrap();
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
                    .unwrap();
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
            Screen::Home => self.home_view(),
            Screen::BoardSelection => self.board_selction_view(),
            Screen::ImageSelection => self.image_selection_view(),
            Screen::DestinationSelection => self.destination_selection_view(),
            Screen::ExtraConfiguration => self.extra_config_view(),
            Screen::Flashing(s) => s.view(),
        }
    }

    const fn theme(&self) -> iced::Theme {
        iced::Theme::Light
    }
}

impl BBImager {
    fn back_home(&mut self) {
        self.search_bar.clear();
        self.screen = Screen::Home;
    }

    fn refresh_destinations(&self) -> Task<BBImagerMessage> {
        let flasher = self
            .boards
            .device(self.selected_board.as_ref().unwrap())
            .flasher;

        Task::perform(
            async move { flasher.destinations().await },
            BBImagerMessage::Destinations,
        )
    }

    fn home_view(&self) -> Element<BBImagerMessage> {
        let choose_device_btn = match &self.selected_board {
            Some(x) => home_btn(x.as_str(), true, iced::Length::Fill),
            None => home_btn("CHOOSE DEVICE", true, iced::Length::Fill),
        }
        .width(iced::Length::Fill)
        .on_press(BBImagerMessage::SwitchScreen(Screen::BoardSelection));

        let choose_image_btn = match &self.selected_image {
            Some(x) => home_btn(x.to_string(), true, iced::Length::Fill),
            None => home_btn(
                "CHOOSE IMAGE",
                self.selected_board.is_some(),
                iced::Length::Fill,
            ),
        }
        .width(iced::Length::Fill)
        .on_press_maybe(if self.selected_board.is_none() {
            None
        } else {
            Some(BBImagerMessage::SwitchScreen(Screen::ImageSelection))
        });

        let choose_dst_btn = match &self.selected_dst {
            Some(x) => home_btn(x.to_string(), true, iced::Length::Fill),
            None => home_btn(
                "CHOOSE DESTINATION",
                self.selected_image.is_some(),
                iced::Length::Fill,
            ),
        }
        .width(iced::Length::Fill)
        .on_press_maybe(if self.selected_image.is_none() {
            None
        } else {
            Some(BBImagerMessage::SwitchScreen(Screen::DestinationSelection))
        });

        let reset_btn = home_btn("RESET", true, iced::Length::Fill)
            .on_press(BBImagerMessage::Reset)
            .width(iced::Length::Fill);

        let next_btn_active = self.selected_board.is_none()
            || self.selected_image.is_none()
            || self.selected_dst.is_none();

        let next_btn = home_btn("NEXT", !next_btn_active, iced::Length::Fill)
            .width(iced::Length::Fill)
            .on_press_maybe(if next_btn_active {
                None
            } else {
                Some(BBImagerMessage::SwitchScreen(Screen::ExtraConfiguration))
            });

        let choice_btn_row = widget::row![
            widget::column![
                text("BeagleBoard").color(iced::Color::WHITE),
                choose_device_btn
            ]
            .spacing(8)
            .width(iced::Length::FillPortion(1))
            .align_x(iced::Alignment::Center),
            widget::column![text("Image").color(iced::Color::WHITE), choose_image_btn]
                .spacing(8)
                .width(iced::Length::FillPortion(1))
                .align_x(iced::Alignment::Center),
            widget::column![
                text("Destination").color(iced::Color::WHITE),
                choose_dst_btn
            ]
            .spacing(8)
            .width(iced::Length::FillPortion(1))
            .align_x(iced::Alignment::Center)
        ]
        .padding(48)
        .spacing(48)
        .width(iced::Length::Fill)
        .align_y(iced::Alignment::Center);

        let action_btn_row = widget::row![
            reset_btn.width(iced::Length::FillPortion(1)),
            widget::horizontal_space().width(iced::Length::FillPortion(5)),
            next_btn.width(iced::Length::FillPortion(1))
        ]
        .padding(48)
        .width(iced::Length::Fill)
        .align_y(iced::Alignment::Center);

        let bottom = widget::container(
            widget::column![
                choice_btn_row.height(iced::Length::FillPortion(1)),
                action_btn_row.height(iced::Length::FillPortion(1))
            ]
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .align_x(iced::Alignment::Center),
        )
        .style(|_| widget::container::background(iced::Color::parse("#aa5137").unwrap()));

        widget::column![helpers::logo(), bottom]
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .align_x(iced::Alignment::Center)
            .into()
    }

    fn board_selction_view(&self) -> Element<BBImagerMessage> {
        let items = self
            .boards
            .devices()
            .filter(|(name, _)| {
                name.to_lowercase()
                    .contains(&self.search_bar.to_lowercase())
            })
            .map(|(name, dev)| {
                let image: Element<BBImagerMessage> =
                    match self.downloader.clone().check_image(&dev.icon) {
                        Some(y) => img_or_svg(y, 100),
                        None => widget::svg(widget::svg::Handle::from_memory(
                            constants::DOWNLOADING_ICON,
                        ))
                        .width(40)
                        .into(),
                    };

                button(
                    widget::row![
                        image,
                        widget::column![
                            text(name).size(18),
                            widget::horizontal_space(),
                            text(dev.description.as_str())
                        ]
                        .padding(5)
                    ]
                    .align_y(iced::Alignment::Center)
                    .spacing(10),
                )
                .width(iced::Length::Fill)
                .on_press(BBImagerMessage::BoardSelected(name.to_string()))
                .style(widget::button::secondary)
            })
            .map(Into::into);

        let items = widget::scrollable(widget::column(items).spacing(10));

        widget::column![self.search_bar(None), widget::horizontal_rule(2), items]
            .spacing(10)
            .padding(10)
            .into()
    }

    fn image_selection_view(&self) -> Element<BBImagerMessage> {
        let board = self.selected_board.as_ref().unwrap();
        let items = self
            .boards
            .images(board)
            .filter(|x| {
                x.name
                    .to_lowercase()
                    .contains(&self.search_bar.to_lowercase())
            })
            .map(|x| {
                let mut row3 =
                    widget::row![text(x.release_date.to_string()), widget::horizontal_space()]
                        .spacing(4)
                        .width(iced::Length::Fill);

                row3 = x
                    .tags
                    .iter()
                    .fold(row3, |acc, t| acc.push(iced_aw::Badge::new(text(t))));

                let icon = match self.downloader.clone().check_image(&x.icon) {
                    Some(y) => img_or_svg(y, 80),
                    None => widget::svg(widget::svg::Handle::from_memory(
                        constants::DOWNLOADING_ICON,
                    ))
                    .into(),
                };

                button(
                    widget::row![
                        icon,
                        widget::column![
                            text(x.name.as_str()).size(18),
                            text(x.description.as_str()),
                            row3
                        ]
                        .padding(5)
                    ]
                    .align_y(iced::Alignment::Center)
                    .spacing(10),
                )
                .width(iced::Length::Fill)
                .on_press(BBImagerMessage::SelectImage(
                    bb_imager::SelectedImage::from(x),
                ))
                .style(widget::button::secondary)
            })
            .chain(std::iter::once(
                button(
                    widget::row![
                        widget::svg(widget::svg::Handle::from_memory(constants::FILE_ADD_ICON))
                            .width(100),
                        text("Use Custom Image").size(18),
                    ]
                    .spacing(10),
                )
                .width(iced::Length::Fill)
                .on_press(BBImagerMessage::SelectLocalImage)
                .style(widget::button::secondary),
            ))
            .map(Into::into);

        widget::column![
            self.search_bar(None),
            widget::horizontal_rule(2),
            widget::scrollable(widget::column(items).spacing(10))
        ]
        .spacing(10)
        .padding(10)
        .into()
    }

    fn destination_selection_view(&self) -> Element<BBImagerMessage> {
        let items = self
            .destinations
            .iter()
            .filter(|x| {
                x.to_string()
                    .to_lowercase()
                    .contains(&self.search_bar.to_lowercase())
            })
            .map(|x| {
                let mut row2 = widget::column![text(x.to_string())];

                if let bb_imager::Destination::SdCard { size, .. } = x {
                    let s = (*size as f32) / (1024.0 * 1024.0 * 1024.0);
                    row2 = row2.push(text(format!("{:.2} GB", s)));
                }

                button(
                    widget::row![
                        widget::svg(widget::svg::Handle::from_memory(constants::USB_ICON))
                            .width(40),
                        row2
                    ]
                    .align_y(iced::Alignment::Center)
                    .spacing(10),
                )
                .width(iced::Length::Fill)
                .on_press(BBImagerMessage::SelectPort(x.clone()))
                .style(widget::button::secondary)
            })
            .map(Into::into);

        widget::column![
            self.search_bar(Some(BBImagerMessage::RefreshDestinations)),
            widget::horizontal_rule(2),
            widget::scrollable(widget::column(items).spacing(10))
        ]
        .spacing(10)
        .padding(10)
        .into()
    }

    fn extra_config_view(&self) -> Element<BBImagerMessage> {
        widget::responsive(|size| {
            let action_btn_row = widget::row![
                home_btn("BACK", true, iced::Length::Fill)
                    .style(widget::button::secondary)
                    .width(iced::Length::FillPortion(1))
                    .on_press(BBImagerMessage::SwitchScreen(Screen::Home)),
                widget::horizontal_space().width(iced::Length::FillPortion(5)),
                home_btn("WRITE", true, iced::Length::Fill)
                    .style(widget::button::secondary)
                    .width(iced::Length::FillPortion(1))
                    .on_press(BBImagerMessage::StartFlashing)
            ]
            .padding(32)
            .width(iced::Length::Fill);

            let form = match self.flashing_config.as_ref().unwrap() {
                bb_imager::FlashingConfig::LinuxSd(x) => self.linux_sd_form(x),
                bb_imager::FlashingConfig::Bcf(x) => widget::column![widget::toggler(!x.verify)
                    .label("Skip Verification")
                    .on_toggle(|y| {
                        BBImagerMessage::UpdateFlashConfig(bb_imager::FlashingConfig::Bcf(
                            x.clone().update_verify(!y),
                        ))
                    })],
                bb_imager::FlashingConfig::Msp430 => widget::column([]),
            }
            .spacing(5);

            widget::column![
                text("Extra Configuration").size(28),
                widget::horizontal_rule(2),
                widget::scrollable(form).height(size.height - 210.0),
                action_btn_row,
            ]
            .spacing(10)
            .padding(10)
            .height(iced::Length::Fill)
            .width(iced::Length::Fill)
            .align_x(iced::Alignment::Center)
            .into()
        })
        .into()
    }

    fn linux_sd_form<'a>(
        &'a self,
        config: &'a bb_imager::FlashingSdLinuxConfig,
    ) -> widget::Column<'a, BBImagerMessage> {
        let xc = config.clone();
        let timezone_box = widget::combo_box(
            self.timezones.as_ref().unwrap(),
            "Timezone",
            config.timezone.as_ref(),
            move |t| {
                let tz = if t.is_empty() { None } else { Some(t) };
                BBImagerMessage::UpdateFlashConfig(bb_imager::FlashingConfig::LinuxSd(
                    xc.clone().update_timezone(tz),
                ))
            },
        )
        .width(200);

        let xc = config.clone();
        let keymap_box = widget::combo_box(
            self.keymaps.as_ref().unwrap(),
            "Keymap",
            config.keymap.as_ref(),
            move |t| {
                let tz = if t.is_empty() { None } else { Some(t) };
                BBImagerMessage::UpdateFlashConfig(bb_imager::FlashingConfig::LinuxSd(
                    xc.clone().update_keymap(tz),
                ))
            },
        )
        .width(200);

        widget::column![
            widget::container(
                widget::toggler(!config.verify)
                    .label("Skip Verification")
                    .on_toggle(|y| {
                        BBImagerMessage::UpdateFlashConfig(bb_imager::FlashingConfig::LinuxSd(
                            config.clone().update_verify(!y),
                        ))
                    })
            )
            .padding(10)
            .width(iced::Length::Fill)
            .style(widget::container::bordered_box),
            widget::container(helpers::input_with_label(
                "Set Hostname",
                "beagle",
                config.hostname.as_deref().unwrap_or_default(),
                |inp| {
                    let h = if inp.is_empty() { None } else { Some(inp) };
                    bb_imager::FlashingConfig::LinuxSd(config.clone().update_hostname(h))
                }
            ))
            .style(widget::container::bordered_box),
            widget::container(helpers::element_with_label(
                "Set Timezone",
                timezone_box.into()
            ))
            .style(widget::container::bordered_box),
            widget::container(helpers::element_with_label("Set Keymap", keymap_box.into()))
                .style(widget::container::bordered_box),
            uname_pass_form(config).width(iced::Length::Fill),
            wifi_form(config).width(iced::Length::Fill)
        ]
    }

    fn search_bar(&self, refresh: Option<BBImagerMessage>) -> Element<BBImagerMessage> {
        let mut row = widget::row![button(
            widget::svg(widget::svg::Handle::from_memory(constants::ARROW_BACK_ICON)).width(22)
        )
        .on_press(BBImagerMessage::SwitchScreen(Screen::Home))
        .style(widget::button::secondary)]
        .spacing(10);

        if let Some(r) = refresh {
            row = row.push(
                button(
                    widget::svg(widget::svg::Handle::from_memory(constants::REFRESH_ICON))
                        .width(22),
                )
                .on_press(r)
                .style(widget::button::secondary),
            );
        }

        row.push(widget::text_input("Search", &self.search_bar).on_input(BBImagerMessage::Search))
            .into()
    }
}

#[derive(Default, Debug, Clone)]
enum Screen {
    #[default]
    Home,
    BoardSelection,
    ImageSelection,
    DestinationSelection,
    ExtraConfiguration,
    Flashing(FlashingScreen),
}

#[derive(Debug, Clone)]
struct FlashingScreen {
    progress: ProgressBarState,
    documentation: String,
    running: bool,
}

impl Default for FlashingScreen {
    fn default() -> Self {
        FlashingScreen {
            progress: ProgressBarState::PREPARING,
            documentation: String::new(),
            running: true,
        }
    }
}

impl FlashingScreen {
    fn new(documentation: String) -> Self {
        Self {
            documentation,
            ..Default::default()
        }
    }

    fn view(&self) -> Element<BBImagerMessage> {
        widget::responsive(|size| {
            let prog_bar = self.progress.bar();

            let btn = if self.running {
                home_btn("CANCEL", true, iced::Length::Shrink)
                    .on_press(BBImagerMessage::CancelFlashing)
            } else {
                home_btn("HOME", true, iced::Length::Shrink)
                    .on_press(BBImagerMessage::SwitchScreen(Screen::Home))
            };

            let bottom = widget::container(
                widget::column![self.about().height(size.height - 410.0), btn, prog_bar]
                    .width(iced::Length::Fill)
                    .height(iced::Length::Fill)
                    .align_x(iced::Alignment::Center),
            )
            .style(|_| widget::container::background(iced::Color::parse("#aa5137").unwrap()));

            widget::column![helpers::logo(), bottom]
                .spacing(10)
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .align_x(iced::Alignment::Center)
                .into()
        })
        .into()
    }

    fn about(&self) -> widget::Container<'_, BBImagerMessage> {
        widget::container(widget::scrollable(widget::rich_text![
            widget::span(constants::BEAGLE_BOARD_ABOUT)
                .link(BBImagerMessage::OpenUrl(
                    "https://www.beagleboard.org/about".into()
                ))
                .color(iced::Color::WHITE),
            widget::span("\n\n"),
            widget::span("For more information, check out our documentation")
                .link(BBImagerMessage::OpenUrl(self.documentation.clone().into()))
                .color(iced::Color::WHITE)
        ]))
        .padding(32)
    }
}

fn img_or_svg<'a>(path: std::path::PathBuf, width: u16) -> Element<'a, BBImagerMessage> {
    let img = std::fs::read(path).unwrap();

    match image::guess_format(&img) {
        Ok(_) => widget::image(widget::image::Handle::from_bytes(img))
            .width(width)
            .height(width)
            .into(),

        Err(_) => widget::svg(widget::svg::Handle::from_memory(img))
            .width(width)
            .height(width)
            .into(),
    }
}

fn timezone() -> widget::combo_box::State<String> {
    let temp = include_str!("../assets/timezones.txt")
        .split_whitespace()
        .map(|x| x.to_string())
        .collect();

    widget::combo_box::State::new(temp)
}

fn keymap() -> widget::combo_box::State<String> {
    let temp = include_str!("../assets/keymap-layouts.txt")
        .split_whitespace()
        .map(|x| x.to_string())
        .collect();

    widget::combo_box::State::new(temp)
}

fn uname_pass_form(
    config: &bb_imager::FlashingSdLinuxConfig,
) -> widget::Container<BBImagerMessage> {
    let mut form = widget::column![widget::toggler(config.user.is_some())
        .label("Configure Username and Password")
        .on_toggle(|t| {
            let c = if t {
                Some((String::new(), String::new()))
            } else {
                None
            };
            BBImagerMessage::UpdateFlashConfig(bb_imager::FlashingConfig::LinuxSd(
                config.clone().update_user(c),
            ))
        })];

    if let Some((u, p)) = &config.user {
        form = form.extend([
            helpers::input_with_label("Username", "username", u, |inp| {
                bb_imager::FlashingConfig::LinuxSd(
                    config.clone().update_user(Some((inp, p.clone()))),
                )
            })
            .into(),
            helpers::input_with_label("Password", "password", p, |inp| {
                bb_imager::FlashingConfig::LinuxSd(
                    config.clone().update_user(Some((u.clone(), inp))),
                )
            })
            .into(),
        ]);
    }

    widget::container(form)
        .padding(10)
        .style(widget::container::bordered_box)
}

fn wifi_form(config: &bb_imager::FlashingSdLinuxConfig) -> widget::Container<BBImagerMessage> {
    let mut form = widget::column![widget::toggler(config.wifi.is_some())
        .label("Configure Wireless LAN")
        .on_toggle(|t| {
            let c = if t {
                Some((String::new(), String::new()))
            } else {
                None
            };
            BBImagerMessage::UpdateFlashConfig(bb_imager::FlashingConfig::LinuxSd(
                config.clone().update_wifi(c),
            ))
        })];

    if let Some((ssid, psk)) = &config.wifi {
        form = form.extend([
            helpers::input_with_label("SSID", "SSID", ssid, |inp| {
                bb_imager::FlashingConfig::LinuxSd(
                    config.clone().update_wifi(Some((inp, psk.clone()))),
                )
            })
            .into(),
            helpers::input_with_label("Password", "password", psk, |inp| {
                bb_imager::FlashingConfig::LinuxSd(
                    config.clone().update_wifi(Some((ssid.clone(), inp))),
                )
            })
            .into(),
        ]);
    }

    widget::container(form)
        .padding(10)
        .style(widget::container::bordered_box)
}

fn home_btn<'a>(
    txt: impl text::IntoFragment<'a>,
    active: bool,
    text_width: iced::Length,
) -> widget::Button<'a, BBImagerMessage> {
    let btn = button(
        text(txt)
            .font(constants::FONT_BOLD)
            .align_x(iced::Alignment::Center)
            .align_y(iced::Alignment::Center)
            .width(text_width),
    )
    .padding(16);

    if active {
        btn.style(|_, _| active_btn_style())
    } else {
        btn.style(|_, _| widget::button::Style {
            background: Some(iced::Color::BLACK.scale_alpha(0.5).into()),
            text_color: iced::Color::BLACK.scale_alpha(0.8),
            ..Default::default()
        })
    }
}

fn active_btn_style() -> widget::button::Style {
    widget::button::Style {
        background: Some(iced::Color::WHITE.into()),
        text_color: iced::Color::parse("#aa5137").unwrap(),
        ..Default::default()
    }
}
