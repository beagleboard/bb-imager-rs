#![windows_subsystem = "windows"]

use std::{borrow::Cow, collections::HashSet, path::PathBuf};

use futures_util::SinkExt;
use helpers::ProgressBarState;
use iced::{
    widget::{self, button, text},
    Element, Task,
};

mod constants;
mod helpers;

fn main() -> iced::Result {
    tracing_subscriber::fmt().init();

    let icon = iced::window::icon::from_file_data(constants::WINDOW_ICON, None).ok();
    assert!(icon.is_some());

    // TODO: Load Config from network
    let config = bb_imager::config::Config::from_json(constants::DEFAULT_CONFIG)
        .expect("Failed to parse config");

    iced::application("BeagleBoard Imager", BBImager::update, BBImager::view)
        .theme(BBImager::theme)
        .run_with(move || BBImager::new(config))
}

#[derive(Default, Debug)]
struct BBImager {
    config: bb_imager::config::Config,
    downloader: bb_imager::download::Downloader,
    screen: Screen,
    selected_board: Option<bb_imager::config::Device>,
    selected_image: Option<bb_imager::common::SelectedImage>,
    selected_dst: Option<bb_imager::Destination>,
    destinations: HashSet<bb_imager::Destination>,
    search_bar: String,
    cancel_flashing: Option<tokio::sync::oneshot::Sender<()>>,
    flashing_config: Option<bb_imager::FlashingConfig>,

    timezones: Option<widget::combo_box::State<String>>,
    keymaps: Option<widget::combo_box::State<String>>,
}

#[derive(Debug, Clone)]
enum BBImagerMessage {
    BoardSelected(Box<bb_imager::config::Device>),
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

    BoardImageDownloaded { index: usize, path: PathBuf },
    OsListImageDownloaded { index: usize, path: PathBuf },

    OpenUrl(Cow<'static, str>),

    Null,
}

impl BBImager {
    fn new(config: bb_imager::config::Config) -> (Self, Task<BBImagerMessage>) {
        let downloader = bb_imager::download::Downloader::default();

        // Fetch all board images
        let board_image = config.devices().iter().enumerate().map(|(index, v)| {
            Task::perform(
                downloader.clone().download(v.icon.clone(), v.icon_sha256),
                move |p| match p {
                    Ok(path) => BBImagerMessage::BoardImageDownloaded { index, path },
                    Err(_) => {
                        tracing::warn!("Failed to fetch image for board {index}");
                        BBImagerMessage::Null
                    }
                },
            )
        });

        let os_image_from_cache = config.os_list.iter().enumerate().map(|(index, v)| {
            let downloader_clone = downloader.clone();
            let icon = v.icon.clone();
            let sha = v.icon_sha256;

            Task::perform(downloader_clone.check_cache(icon, sha), move |p| match p {
                Some(path) => BBImagerMessage::OsListImageDownloaded { index, path },
                None => BBImagerMessage::Null,
            })
        });

        (
            Self {
                config: config.clone(),
                downloader: downloader.clone(),
                timezones: Some(timezone()),
                keymaps: Some(keymap()),
                ..Default::default()
            },
            Task::batch(board_image.chain(os_image_from_cache)),
        )
    }

    fn update(&mut self, message: BBImagerMessage) -> Task<BBImagerMessage> {
        match message {
            BBImagerMessage::BoardSelected(x) => {
                // Reset any previously selected values
                self.selected_dst.take();
                self.selected_image.take();
                self.destinations.clear();

                self.selected_board = Some(*x.clone());
                self.back_home();

                let jobs = self
                    .config
                    .os_list
                    .iter()
                    .enumerate()
                    .filter(|(_, x)| x.icon_local.is_none())
                    .filter(|(_, v)| v.devices.contains(&x.name))
                    .map(|(index, v)| {
                        Task::perform(
                            self.downloader
                                .clone()
                                .download(v.icon.clone(), v.icon_sha256),
                            move |p| match p {
                                Ok(path) => BBImagerMessage::OsListImageDownloaded { index, path },
                                Err(e) => {
                                    tracing::warn!(
                                        "Failed to download image for os {index} with error {e}"
                                    );
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
                let (name, extensions) =
                    self.selected_board.as_ref().unwrap().flasher.file_filter();
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
                        self.flashing_config =
                            Some(self.selected_board.as_ref().unwrap().flasher.into());
                    }
                    _ => {}
                }
            }
            BBImagerMessage::Search(x) => {
                self.search_bar = x;
            }
            BBImagerMessage::BoardImageDownloaded { index, path } => {
                self.config.imager.devices[index].icon_local = Some(path);
            }
            BBImagerMessage::OsListImageDownloaded { index, path } => {
                self.config.os_list[index].icon_local = Some(path);
            }
            BBImagerMessage::CancelFlashing => {
                if let Some(tx) = self.cancel_flashing.take() {
                    let _ = tx.send(());
                }
            }
            BBImagerMessage::StartFlashing => {
                self.screen = Screen::Flashing(FlashingScreen::new(
                    self.selected_board
                        .as_ref()
                        .unwrap()
                        .documentation
                        .to_string(),
                ));

                let dst = self.selected_dst.clone().expect("No destination selected");
                let img = self.selected_image.clone().unwrap();
                let downloader = self.downloader.clone();
                let config = self.flashing_config.clone().unwrap();
                let (tx, cancel_rx) = tokio::sync::oneshot::channel();

                self.cancel_flashing = Some(tx);

                let s = iced::stream::channel(20, move |mut chan| async move {
                    let _ = chan
                        .send(BBImagerMessage::ProgressBar(ProgressBarState::PREPARING))
                        .await;

                    let (tx, mut rx) = tokio::sync::mpsc::channel(20);

                    let flash_task = tokio::spawn(
                        bb_imager::common::Flasher::new(img, dst, downloader, tx, config)
                            .download_flash_customize(),
                    );

                    let mut chan_clone = chan.clone();
                    let chan_task = tokio::spawn(async move {
                        while let Some(progress) = rx.recv().await {
                            let _ =
                                chan_clone.try_send(BBImagerMessage::ProgressBar(progress.into()));
                        }
                    });

                    tokio::select! {
                        _ = cancel_rx => {
                            flash_task.abort();
                            let _ = chan.send(BBImagerMessage::StopFlashing(ProgressBarState::fail(
                                "Flashing Cancelled by User",
                            ))).await;
                        },
                        _ = chan_task => {
                            let res = flash_task.await.unwrap();
                            let _ = match res {
                                Ok(_) => chan.send(BBImagerMessage::StopFlashing(ProgressBarState::FLASHING_SUCCESS)),
                                Err(e) => chan.send(BBImagerMessage::StopFlashing(ProgressBarState::fail(
                                    format!("Flashing Failed {e}"),
                                ))),
                            }
                            .await;
                        }
                    };
                });

                return Task::stream(s);
            }
            BBImagerMessage::StopFlashing(x) => {
                let content = x.content();

                let progress_task = Task::done(BBImagerMessage::ProgressBar(x));
                let notification_task = Task::perform(
                    async move {
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
                    },
                    |_| BBImagerMessage::Null,
                );

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
                return Task::perform(
                    async move {
                        let res = webbrowser::open(&x);
                        tracing::info!("Open Url Resp {res:?}");
                    },
                    |_| BBImagerMessage::Null,
                );
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
        iced::Theme::Oxocarbon
    }
}

impl BBImager {
    fn back_home(&mut self) {
        self.search_bar.clear();
        self.screen = Screen::Home;
    }

    fn refresh_destinations(&self) -> Task<BBImagerMessage> {
        let flasher = self.selected_board.clone().unwrap().flasher;

        Task::perform(async move { flasher.destinations().await }, |x| {
            BBImagerMessage::Destinations(x)
        })
    }

    fn home_view(&self) -> Element<BBImagerMessage> {
        let choose_device_btn = match &self.selected_board {
            Some(x) => home_btn(x.name.as_str()),
            None => home_btn("CHOOSE DEVICE"),
        }
        .on_press(BBImagerMessage::SwitchScreen(Screen::BoardSelection));

        let choose_image_btn = match &self.selected_image {
            Some(x) => home_btn(x.to_string()),
            None => home_btn("CHOOSE IMAGE"),
        }
        .on_press_maybe(if self.selected_board.is_none() {
            None
        } else {
            Some(BBImagerMessage::SwitchScreen(Screen::ImageSelection))
        });

        let choose_dst_btn = match &self.selected_dst {
            Some(x) => home_btn(x.to_string()),
            None => home_btn("CHOOSE DESTINATION"),
        }
        .on_press_maybe(if self.selected_image.is_none() {
            None
        } else {
            Some(BBImagerMessage::SwitchScreen(Screen::DestinationSelection))
        });

        let reset_btn = home_btn("RESET").on_press(BBImagerMessage::Reset);

        let next_btn = home_btn("NEXT").on_press_maybe(
            if self.selected_board.is_none()
                || self.selected_image.is_none()
                || self.selected_dst.is_none()
            {
                None
            } else {
                Some(BBImagerMessage::SwitchScreen(Screen::ExtraConfiguration))
            },
        );

        let choice_btn_row = widget::row![
            widget::column![text("Board"), choose_device_btn]
                .spacing(8)
                .width(iced::Length::FillPortion(1))
                .align_x(iced::Alignment::Center),
            widget::column![text("Image"), choose_image_btn]
                .spacing(8)
                .width(iced::Length::FillPortion(1))
                .align_x(iced::Alignment::Center),
            widget::column![text("Destination"), choose_dst_btn]
                .spacing(8)
                .width(iced::Length::FillPortion(1))
                .align_x(iced::Alignment::Center)
        ]
        .padding(64)
        .spacing(48)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .align_y(iced::Alignment::Center);

        let action_btn_row = widget::row![
            reset_btn.width(iced::Length::FillPortion(1)),
            widget::horizontal_space().width(iced::Length::FillPortion(5)),
            next_btn.width(iced::Length::FillPortion(1))
        ]
        .padding(64)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .align_y(iced::Alignment::Center);

        widget::column![helpers::logo(), choice_btn_row, action_btn_row]
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .align_x(iced::Alignment::Center)
            .into()
    }

    fn board_selction_view(&self) -> Element<BBImagerMessage> {
        let items = self
            .config
            .devices()
            .iter()
            .filter(|x| {
                x.name
                    .to_lowercase()
                    .contains(&self.search_bar.to_lowercase())
            })
            .map(|x| {
                let image: Element<BBImagerMessage> = match &x.icon_local {
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
                            text(x.name.as_str()).size(18),
                            widget::horizontal_space(),
                            text(x.description.as_str())
                        ]
                        .padding(5)
                    ]
                    .align_y(iced::Alignment::Center)
                    .spacing(10),
                )
                .width(iced::Length::Fill)
                .on_press(BBImagerMessage::BoardSelected(Box::new(x.clone())))
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
            .config
            .images_by_device(board)
            .filter(|x| {
                x.name
                    .to_lowercase()
                    .contains(&self.search_bar.to_lowercase())
            })
            .map(|x| {
                let mut row3 =
                    widget::row![text(x.release_date.to_string()), widget::horizontal_space(),]
                        .width(iced::Length::Fill);

                row3 = x
                    .tags
                    .iter()
                    .fold(row3, |acc, t| acc.push(iced_aw::Badge::new(text(t))));

                let icon = match &x.icon_local {
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
        let action_btn_row = widget::row![
            button("BACK")
                .on_press(BBImagerMessage::SwitchScreen(Screen::Home))
                .padding(10),
            widget::horizontal_space(),
            button("WRITE")
                .on_press(BBImagerMessage::StartFlashing)
                .padding(10)
        ]
        .padding(40)
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
        .spacing(5)
        .width(iced::Length::Fill);

        let form = widget::scrollable(form.push(action_btn_row));

        widget::column![
            text("Extra Configuration").size(28),
            widget::horizontal_rule(2),
            form,
        ]
        .spacing(10)
        .padding(10)
        .width(iced::Length::Fill)
        .align_x(iced::Alignment::Center)
        .into()
    }

    fn linux_sd_form<'a>(
        &'a self,
        config: &'a bb_imager::FlashingSdLinuxConfig,
    ) -> widget::Column<BBImagerMessage> {
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
            uname_pass_form(config),
            wifi_form(config)
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
}

impl Default for FlashingScreen {
    fn default() -> Self {
        FlashingScreen {
            progress: ProgressBarState::PREPARING,
            documentation: String::new(),
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
        let prog_bar = self.progress.bar();

        let btn = if self.progress.running() {
            button("CANCEL").on_press(BBImagerMessage::CancelFlashing)
        } else {
            button("HOME").on_press(BBImagerMessage::SwitchScreen(Screen::Home))
        }
        .padding(10);

        widget::column![
            helpers::logo(),
            widget::vertical_space(),
            self.about(),
            widget::vertical_space(),
            btn,
            prog_bar
        ]
        .spacing(10)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .align_x(iced::Alignment::Center)
        .into()
    }

    fn about(&self) -> Element<'_, BBImagerMessage> {
        widget::container(widget::rich_text![
            widget::span(constants::BEAGLE_BOARD_ABOUT).link(BBImagerMessage::OpenUrl(
                "https://www.beagleboard.org/about".into()
            )),
            widget::span("\n\n"),
            widget::span("For more information, check out our documentation")
                .link(BBImagerMessage::OpenUrl(self.documentation.clone().into()))
        ])
        .padding(30)
        .into()
    }
}

fn img_or_svg(path: &std::path::Path, width: u16) -> Element<BBImagerMessage> {
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
    let temp = include_str!("../../assets/timezones.txt")
        .split_whitespace()
        .map(|x| x.to_string())
        .collect();

    widget::combo_box::State::new(temp)
}

fn keymap() -> widget::combo_box::State<String> {
    let temp = include_str!("../../assets/keymap-layouts.txt")
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

fn home_btn<'a>(txt: impl text::IntoFragment<'a>) -> widget::Button<'a, BBImagerMessage> {
    button(
        text(txt)
            .align_x(iced::Alignment::Center)
            .align_y(iced::Alignment::Center)
            .width(iced::Length::Fill),
    )
    .width(iced::Length::Fill)
    .padding(16)
}
