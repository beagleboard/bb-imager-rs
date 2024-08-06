use std::{borrow::Cow, collections::HashSet, path::PathBuf};

use futures_util::SinkExt;
use iced::{
    executor, theme,
    widget::{self, button, text},
    Application, Command, Element, Settings,
};

// TODO: Load Config from network
const CONFIG: &[u8] = include_bytes!("../../config.json");

const WINDOW_ICON: &[u8] = include_bytes!("../icon.png");
const BB_BANNER: &[u8] = include_bytes!("../../icons/bb-banner.png");
const ARROW_BACK_ICON: &[u8] = include_bytes!("../../icons/arrow-back.svg");
const DOWNLOADING_ICON: &[u8] = include_bytes!("../../icons/downloading.svg");
const FILE_ADD_ICON: &[u8] = include_bytes!("../../icons/file-add.svg");
const USB_ICON: &[u8] = include_bytes!("../../icons/usb.svg");
const REFRESH_ICON: &[u8] = include_bytes!("../../icons/refresh.svg");

fn main() -> iced::Result {
    tracing_subscriber::fmt().init();

    let icon = iced::window::icon::from_file_data(WINDOW_ICON, None).ok();

    assert!(icon.is_some());

    let config = bb_imager::config::Config::from_json(CONFIG).expect("Failed to parse config");

    let settings = Settings {
        window: iced::window::Settings {
            size: iced::Size::new(800.0, 500.0),
            icon,
            ..Default::default()
        },
        flags: Flags { config },
        ..Default::default()
    };

    BBImager::run(settings)
}

#[derive(Default, Debug)]
struct BBImager {
    config: bb_imager::config::Config,
    state: Option<bb_imager::State>,
    downloader: bb_imager::download::Downloader,
    screen: Screen,
    selected_board: Option<bb_imager::config::Device>,
    selected_image: Option<bb_imager::common::SelectedImage>,
    selected_dst: Option<bb_imager::Destination>,
    destinations: HashSet<bb_imager::Destination>,
    search_bar: String,
    progress_bar: ProgressBarState,
    flashing: bool,
}

#[derive(Default, Debug)]
struct Flags {
    config: bb_imager::config::Config,
}

#[derive(Debug, Clone)]
enum BBImagerMessage {
    InitState(bb_imager::State),
    BoardSelected(Box<bb_imager::config::Device>),
    SelectImage(Option<Box<bb_imager::config::OsList>>),
    SelectPort(bb_imager::Destination),
    ProgressBar(ProgressBarState),
    SwitchScreen(Screen),
    Search(String),
    Destinations(HashSet<bb_imager::Destination>),
    RefreshDestinations,
    Reset,

    StartFlashing,
    StopFlashing(ProgressBarState),

    BoardImageDownloaded { index: usize, path: PathBuf },
    OsListImageDownloaded { index: usize, path: PathBuf },

    UnrecoverableError(String),
    Null,
}

#[derive(Clone, Debug, Default)]
struct ProgressBarState {
    label: Cow<'static, str>,
    progress: f32,
    state: ProgressBarStatus,
}

impl ProgressBarState {
    fn new(label: impl Into<Cow<'static, str>>, progress: f32, state: ProgressBarStatus) -> Self {
        Self {
            label: label.into(),
            progress,
            state,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
enum ProgressBarStatus {
    #[default]
    Normal,
    Success,
    Fail,
    Loading,
}

impl From<ProgressBarStatus> for widget::theme::ProgressBar {
    fn from(value: ProgressBarStatus) -> Self {
        match value {
            ProgressBarStatus::Normal => widget::theme::ProgressBar::Primary,
            ProgressBarStatus::Success => widget::theme::ProgressBar::Success,
            ProgressBarStatus::Fail => widget::theme::ProgressBar::Danger,
            // TODO: Add better loading theme
            ProgressBarStatus::Loading => widget::theme::ProgressBar::Primary,
        }
    }
}

impl Application for BBImager {
    type Message = BBImagerMessage;
    type Executor = executor::Default;
    type Flags = Flags;
    type Theme = theme::Theme;

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let downloader = bb_imager::download::Downloader::default();

        // Fetch all board images
        let board_image = flags.config.devices().iter().enumerate().map(|(index, v)| {
            Command::perform(
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

        let os_image_from_cache = flags.config.os_list.iter().enumerate().map(|(index, v)| {
            let downloader_clone = downloader.clone();
            let icon = v.icon.clone();
            let sha = v.icon_sha256;

            Command::perform(
                async move {
                    tokio::task::spawn_blocking(move || downloader_clone.check_cache(icon, sha))
                        .await
                        .unwrap()
                },
                move |p| match p {
                    Some(path) => BBImagerMessage::OsListImageDownloaded { index, path },
                    None => BBImagerMessage::Null,
                },
            )
        });

        let state_cmd = Command::perform(bb_imager::State::new(), |x| match x {
            Ok(x) => BBImagerMessage::InitState(x),
            Err(e) => BBImagerMessage::UnrecoverableError(e.to_string()),
        });

        (
            Self {
                config: flags.config.clone(),
                downloader: downloader.clone(),
                ..Default::default()
            },
            Command::batch(board_image.chain(os_image_from_cache).chain([state_cmd])),
        )
    }

    fn title(&self) -> String {
        String::from("BeagleBoard Imager")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
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
                        Command::perform(
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

                return Command::batch(jobs.chain([self.refresh_destinations()]));
            }
            BBImagerMessage::ProgressBar(s) => self.progress_bar = s,
            BBImagerMessage::SelectImage(x) => {
                self.selected_image = match x {
                    Some(y) => Some(bb_imager::common::SelectedImage::remote(
                        y.name,
                        y.url,
                        y.extract_sha256,
                        y.extract_path,
                    )),
                    None => {
                        let (name, extensions) =
                            self.selected_board.as_ref().unwrap().flasher.file_filter();
                        rfd::FileDialog::new()
                            .add_filter(name, extensions)
                            .pick_file()
                            .map(bb_imager::common::SelectedImage::local)
                    }
                };
                if self.selected_image.is_some() {
                    self.back_home();
                }
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
                    Screen::BoardSelection => {}
                    Screen::ImageSelection => {}
                    Screen::DestinationSelection => {
                        return self.refresh_destinations();
                    }
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
            BBImagerMessage::StartFlashing => {
                self.flashing = true;

                let flasher = self
                    .selected_board
                    .clone()
                    .expect("No board selected")
                    .flasher;
                let dst = self.selected_dst.clone().expect("No destination selected");
                let img = self.selected_image.clone().unwrap();
                let state = self.state.clone().unwrap();
                let downloader = self.downloader.clone();

                return iced::command::channel(20, move |mut chan| async move {
                    let _ = chan
                        .send(BBImagerMessage::ProgressBar(ProgressBarState::new(
                            "Preparing...",
                            0.5,
                            ProgressBarStatus::Loading,
                        )))
                        .await;

                    let (tx, rx) = std::sync::mpsc::channel();

                    let task = tokio::spawn(async move {
                        bb_imager::common::download_and_flash(
                            img, dst, flasher, state, downloader, tx, true,
                        )
                        .await
                    });

                    let mut chan_clone = chan.clone();
                    tokio::task::spawn_blocking(move || {
                        while let Ok(progress) = rx.recv() {
                            let message = match progress {
                                bb_imager::DownloadFlashingStatus::Preparing => {
                                    BBImagerMessage::ProgressBar(ProgressBarState::new(
                                        "Preparing...",
                                        0.5,
                                        ProgressBarStatus::Loading,
                                    ))
                                }
                                bb_imager::DownloadFlashingStatus::DownloadingProgress(p) => {
                                    BBImagerMessage::ProgressBar(ProgressBarState::new(
                                        format!(
                                            "Downloading Image... {}%",
                                            (p * 100.0).round() as usize
                                        ),
                                        p,
                                        ProgressBarStatus::Normal,
                                    ))
                                }
                                bb_imager::DownloadFlashingStatus::FlashingProgress(p) => {
                                    BBImagerMessage::ProgressBar(ProgressBarState::new(
                                        format!("Flashing... {}%", (p * 100.0).round() as usize),
                                        p,
                                        ProgressBarStatus::Normal,
                                    ))
                                }
                                bb_imager::DownloadFlashingStatus::Verifying => {
                                    BBImagerMessage::ProgressBar(ProgressBarState::new(
                                        "Verifying...",
                                        0.5,
                                        ProgressBarStatus::Loading,
                                    ))
                                }
                                bb_imager::DownloadFlashingStatus::VerifyingProgress(p) => {
                                    BBImagerMessage::ProgressBar(ProgressBarState::new(
                                        format!("Verifying... {}%", (p * 100.0).round() as usize),
                                        p,
                                        ProgressBarStatus::Normal,
                                    ))
                                }
                                bb_imager::DownloadFlashingStatus::Finished => {
                                    BBImagerMessage::StopFlashing(ProgressBarState::new(
                                        "Flashing Successful...",
                                        1.0,
                                        ProgressBarStatus::Success,
                                    ))
                                }
                            };

                            let _ = chan_clone.try_send(message);
                        }
                    });

                    let res = task.await.unwrap();

                    let _ = match res {
                        Ok(_) => chan.send(BBImagerMessage::StopFlashing(ProgressBarState::new(
                            "Flashing Successful...",
                            1.0,
                            ProgressBarStatus::Success,
                        ))),
                        Err(e) => chan.send(BBImagerMessage::StopFlashing(ProgressBarState::new(
                            format!("Flashing Failed... {e}"),
                            1.0,
                            ProgressBarStatus::Fail,
                        ))),
                    }
                    .await;
                });
            }
            BBImagerMessage::StopFlashing(x) => {
                self.flashing = false;
                self.progress_bar = x;
            }
            BBImagerMessage::Destinations(x) => {
                self.destinations = x;
            }
            BBImagerMessage::InitState(s) => {
                self.state = Some(s);
            }
            BBImagerMessage::UnrecoverableError(e) => {
                panic!("Encounterd unrecoverable error {e}");
            }
            BBImagerMessage::RefreshDestinations => {
                return self.refresh_destinations();
            }

            BBImagerMessage::Null => {}
        };

        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
        match self.screen {
            Screen::Home => self.home_view(),
            Screen::BoardSelection => self.board_selction_view(),
            Screen::ImageSelection => self.image_selection_view(),
            Screen::DestinationSelection => self.destination_selection_view(),
        }
    }

    fn theme(&self) -> Self::Theme {
        iced::Theme::KanagawaLotus
    }
}

impl BBImager {
    fn back_home(&mut self) {
        self.search_bar.clear();
        self.screen = Screen::Home;
    }

    fn refresh_destinations(&self) -> Command<BBImagerMessage> {
        let flasher = self.selected_board.clone().unwrap().flasher;

        Command::perform(async move { flasher.destinations().await }, |x| {
            BBImagerMessage::Destinations(x)
        })
    }

    fn home_view(&self) -> Element<BBImagerMessage> {
        const HOME_BTN_PADDING: u16 = 10;

        let logo = widget::image(widget::image::Handle::from_memory(BB_BANNER)).width(500);

        let choose_device_btn = match &self.selected_board {
            Some(x) => button(x.name.as_str()),
            None => button("CHOOSE DEVICE"),
        }
        .padding(HOME_BTN_PADDING)
        .on_press_maybe(if self.flashing {
            None
        } else {
            Some(BBImagerMessage::SwitchScreen(Screen::BoardSelection))
        });

        let choose_image_btn = match &self.selected_image {
            Some(x) => button(text(x)),
            None => button("CHOOSE IMAGE"),
        }
        .padding(HOME_BTN_PADDING)
        .on_press_maybe(if self.flashing || self.selected_board.is_none() {
            None
        } else {
            Some(BBImagerMessage::SwitchScreen(Screen::ImageSelection))
        });

        let choose_dst_btn = match &self.selected_dst {
            Some(x) => button(x.name.as_str()),
            None => button("CHOOSE DESTINATION"),
        }
        .padding(HOME_BTN_PADDING)
        .on_press_maybe(if self.flashing || self.selected_image.is_none() {
            None
        } else {
            Some(BBImagerMessage::SwitchScreen(Screen::DestinationSelection))
        });

        let reset_btn =
            button("RESET")
                .padding(HOME_BTN_PADDING)
                .on_press_maybe(if self.flashing {
                    None
                } else {
                    Some(BBImagerMessage::Reset)
                });

        let write_btn = button("WRITE").padding(HOME_BTN_PADDING).on_press_maybe(
            if self.selected_board.is_none()
                || self.selected_image.is_none()
                || self.selected_dst.is_none()
            {
                None
            } else {
                Some(BBImagerMessage::StartFlashing)
            },
        );

        let choice_btn_row = widget::row![
            widget::column![text("Board"), choose_device_btn]
                .spacing(5)
                .align_items(iced::Alignment::Center),
            widget::horizontal_space(),
            widget::column![text("Image"), choose_image_btn]
                .spacing(5)
                .align_items(iced::Alignment::Center),
            widget::horizontal_space(),
            widget::column![text("Destination"), choose_dst_btn]
                .spacing(5)
                .align_items(iced::Alignment::Center)
        ]
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .align_items(iced::Alignment::Center);

        let action_btn_row = widget::row![reset_btn, widget::horizontal_space(), write_btn]
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .align_items(iced::Alignment::Center);

        let (progress_label, progress_bar) = self.progress();

        widget::column![
            logo,
            choice_btn_row,
            action_btn_row,
            progress_label,
            progress_bar
        ]
        .spacing(5)
        .padding(64)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .align_items(iced::Alignment::Center)
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
                    None => widget::svg(widget::svg::Handle::from_memory(DOWNLOADING_ICON))
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
                    .align_items(iced::Alignment::Center)
                    .spacing(10),
                )
                .width(iced::Length::Fill)
                .on_press(BBImagerMessage::BoardSelected(Box::new(x.clone())))
                .style(theme::Button::Secondary)
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
                let mut row3 = widget::row![text(x.release_date), widget::horizontal_space(),]
                    .width(iced::Length::Fill);

                row3 = x
                    .tags
                    .iter()
                    .fold(row3, |acc, t| acc.push(iced_aw::Badge::new(text(t))));

                let icon = match &x.icon_local {
                    Some(y) => img_or_svg(y, 80),
                    None => widget::svg(widget::svg::Handle::from_memory(DOWNLOADING_ICON)).into(),
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
                    .align_items(iced::Alignment::Center)
                    .spacing(10),
                )
                .width(iced::Length::Fill)
                .on_press(BBImagerMessage::SelectImage(Some(Box::new(x.clone()))))
                .style(theme::Button::Secondary)
            })
            .chain(std::iter::once(
                button(
                    widget::row![
                        widget::svg(widget::svg::Handle::from_memory(FILE_ADD_ICON)).width(100),
                        text("Use Custom Image").size(18),
                    ]
                    .spacing(10),
                )
                .width(iced::Length::Fill)
                .on_press(BBImagerMessage::SelectImage(None))
                .style(theme::Button::Secondary),
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
                x.name
                    .to_lowercase()
                    .contains(&self.search_bar.to_lowercase())
            })
            .map(|x| {
                let mut row2 = widget::column![text(x.name.as_str()),];

                if let Some(size) = x.size {
                    let s = (size as f32) / (1024.0 * 1024.0 * 1024.0);
                    row2 = row2.push(text(format!("{:.2} GB", s)));
                }

                button(
                    widget::row![
                        widget::svg(widget::svg::Handle::from_memory(USB_ICON)).width(40),
                        row2
                    ]
                    .align_items(iced::Alignment::Center)
                    .spacing(10),
                )
                .width(iced::Length::Fill)
                .on_press(BBImagerMessage::SelectPort(x.clone()))
                .style(theme::Button::Secondary)
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

    fn search_bar(&self, refresh: Option<BBImagerMessage>) -> Element<BBImagerMessage> {
        let mut row = widget::row![button(
            widget::svg(widget::svg::Handle::from_memory(ARROW_BACK_ICON)).width(22)
        )
        .on_press(BBImagerMessage::SwitchScreen(Screen::Home))
        .style(theme::Button::Secondary)]
        .spacing(10);

        if let Some(r) = refresh {
            row = row.push(
                button(widget::svg(widget::svg::Handle::from_memory(REFRESH_ICON)).width(22))
                    .on_press(r)
                    .style(theme::Button::Secondary),
            );
        }

        row.push(widget::text_input("Search", &self.search_bar).on_input(BBImagerMessage::Search))
            .into()
    }

    fn progress(&self) -> (widget::Text, widget::ProgressBar) {
        use std::ops::RangeInclusive;
        use theme::ProgressBar;
        use widget::progress_bar;

        const RANGE: RangeInclusive<f32> = (0.0)..=1.0;

        (
            text(self.progress_bar.label.clone()),
            progress_bar(RANGE, self.progress_bar.progress)
                .height(10)
                .style(ProgressBar::from(self.progress_bar.state)),
        )
    }
}

#[derive(Default, Debug, Clone, Copy)]
enum Screen {
    #[default]
    Home,
    BoardSelection,
    ImageSelection,
    DestinationSelection,
}

fn img_or_svg(path: &std::path::Path, width: u16) -> Element<BBImagerMessage> {
    let img = std::fs::read(path).unwrap();

    match image::guess_format(&img) {
        Ok(_) => widget::image(widget::image::Handle::from_memory(img))
            .width(width)
            .height(width)
            .into(),

        Err(_) => widget::svg(widget::svg::Handle::from_memory(img))
            .width(width)
            .height(width)
            .into(),
    }
}
