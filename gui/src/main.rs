use std::{collections::HashSet, path::PathBuf};

use bb_imager::{DownloadStatus, FlashingStatus};
use futures_util::TryStreamExt;
use iced::{
    executor,
    futures::Stream,
    theme,
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
    selected_image: Option<OsImage>,
    selected_dst: Option<bb_imager::Destination>,
    download_status: Option<Result<DownloadStatus, String>>,
    flashing_status: Option<Result<FlashingStatus, String>>,
    destinations: HashSet<bb_imager::Destination>,
    search_bar: String,
    unrecoverable_error: Option<String>,
}

#[derive(Default, Debug)]
struct Flags {
    config: bb_imager::config::Config,
}

#[derive(Debug, Clone)]
enum OsImage {
    Local(PathBuf),
    Remote(bb_imager::config::OsList),
}

impl std::fmt::Display for OsImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OsImage::Local(p) => write!(f, "{}", p.file_name().unwrap().to_string_lossy()),
            OsImage::Remote(r) => write!(f, "{}", r.name),
        }
    }
}

#[derive(Debug, Clone)]
enum BBImagerMessage {
    BoardSelected(bb_imager::config::Device),
    SelectImage(Option<bb_imager::config::OsList>),
    SelectPort(bb_imager::Destination),
    StartFlashing,
    FlashImage {
        path: PathBuf,
        inner_path: Option<String>,
        sha256: Option<[u8; 32]>,
    },

    DownloadStatus(Result<DownloadStatus, String>),
    FlashingStatus(Result<FlashingStatus, String>),
    Reset,

    BoardSectionPage,
    ImageSelectionPage,
    DestinationSelectionPage,
    HomePage,

    Search(String),
    BoardImageDownloaded {
        index: usize,
        path: PathBuf,
    },
    BoardImageDownloadFailed {
        index: usize,
        error: String,
    },

    OsListImageDownloaded {
        index: usize,
        path: PathBuf,
    },
    OsListDownloadFailed {
        index: usize,
        error: String,
    },

    Destinations(Result<HashSet<bb_imager::Destination>, String>),
    InitState(bb_imager::State),

    UnrecoverableError(String),
    Null,
}

impl Application for BBImager {
    type Message = BBImagerMessage;
    type Executor = executor::Default;
    type Flags = Flags;
    type Theme = theme::Theme;

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let downloader = bb_imager::download::Downloader::default();

        let board_image_from_cache = flags.config.devices().iter().enumerate().map(|(index, v)| {
            Command::perform(
                downloader
                    .clone()
                    .check_cache(v.icon.clone(), v.icon_sha256),
                move |p| match p {
                    Some(path) => BBImagerMessage::BoardImageDownloaded { index, path },
                    None => BBImagerMessage::Null,
                },
            )
        });

        let os_image_from_cache = flags.config.os_list.iter().enumerate().map(|(index, v)| {
            Command::perform(
                downloader
                    .clone()
                    .check_cache(v.icon.clone(), v.icon_sha256),
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
            Command::batch(
                board_image_from_cache
                    .chain(os_image_from_cache)
                    .chain([state_cmd]),
            ),
        )
    }

    fn title(&self) -> String {
        String::from("BeagleBoard Imager")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            BBImagerMessage::BoardSelected(x) => {
                self.selected_board = Some(x);
                self.back_home();

                let flasher = self.selected_board.clone().unwrap().flasher;
                let state = self.state.clone().unwrap();

                return Command::perform(async move { flasher.destinations(state).await }, |x| {
                    BBImagerMessage::Destinations(x.map_err(|y| y.to_string()))
                });
            }
            BBImagerMessage::SelectImage(x) => {
                self.selected_image = match x {
                    Some(y) => Some(OsImage::Remote(y)),
                    None => rfd::FileDialog::new()
                        .add_filter("firmware", &["bin"])
                        .pick_file()
                        .map(OsImage::Local),
                };
                self.back_home();
            }
            BBImagerMessage::SelectPort(x) => {
                self.selected_dst = Some(x);
                self.back_home();
            }
            BBImagerMessage::FlashImage {
                path,
                inner_path,
                sha256,
            } => {
                let board = self.selected_board.clone().expect("No board selected");
                let dst = self.selected_dst.clone().expect("No destination selected");

                tracing::info!("Start flashing image {:?}", path);
                let stream = Command::run(
                    flash_helper(
                        path,
                        inner_path,
                        sha256,
                        board,
                        dst,
                        self.state.clone().unwrap(),
                    ),
                    |x| BBImagerMessage::FlashingStatus(x.map_err(|e| e.to_string())),
                );

                return Command::batch([
                    Command::perform(std::future::ready(FlashingStatus::Preparing), |x| {
                        BBImagerMessage::FlashingStatus(Ok(x))
                    }),
                    stream,
                ]);
            }
            BBImagerMessage::FlashingStatus(x) => {
                self.flashing_status = Some(x.map_err(|e| e.to_string()));
            }
            BBImagerMessage::Reset => {
                self.selected_dst.take();
                self.selected_image.take();
                self.selected_board.take();
                self.unrecoverable_error.take();
                self.search_bar.clear();
                self.download_status.take();
                self.flashing_status.take();
                self.destinations.clear();
            }
            BBImagerMessage::HomePage => {
                self.back_home();
            }
            BBImagerMessage::BoardSectionPage => {
                self.screen = Screen::BoardSelection;
                let jobs = self
                    .config
                    .devices()
                    .iter()
                    .enumerate()
                    .filter(|(_, x)| x.icon_local.is_none())
                    .map(|(index, v)| {
                        Command::perform(
                            self.downloader
                                .clone()
                                .download(v.icon.clone(), v.icon_sha256),
                            move |p| match p {
                                Ok(path) => BBImagerMessage::BoardImageDownloaded { index, path },
                                Err(e) => BBImagerMessage::BoardImageDownloadFailed {
                                    index,
                                    error: e.to_string(),
                                },
                            },
                        )
                    });

                return Command::batch(jobs);
            }
            BBImagerMessage::ImageSelectionPage => {
                self.screen = Screen::ImageSelection;
                let board = self.selected_board.as_ref().unwrap().name.clone();
                let jobs = self
                    .config
                    .os_list
                    .iter()
                    .enumerate()
                    .filter(|(_, x)| x.icon_local.is_none())
                    .filter(|(_, v)| v.devices.contains(&board))
                    .map(|(index, v)| {
                        Command::perform(
                            self.downloader
                                .clone()
                                .download(v.icon.clone(), v.icon_sha256),
                            move |p| match p {
                                Ok(path) => BBImagerMessage::OsListImageDownloaded { index, path },
                                Err(e) => BBImagerMessage::OsListDownloadFailed {
                                    index,
                                    error: e.to_string(),
                                },
                            },
                        )
                    });

                return Command::batch(jobs);
            }
            BBImagerMessage::DestinationSelectionPage => {
                self.screen = Screen::DestinationSelection;
            }

            BBImagerMessage::Search(x) => {
                self.search_bar = x;
            }
            BBImagerMessage::BoardImageDownloaded { index, path } => {
                tracing::info!("Successfully downloaded to {:?}", path);
                self.config.imager.devices[index].icon_local = Some(path);
            }
            BBImagerMessage::BoardImageDownloadFailed { index, error } => {
                tracing::warn!(
                    "Failed to fetch icon for {:?}, Error: {error}",
                    self.config.imager.devices[index]
                );
            }
            BBImagerMessage::OsListImageDownloaded { index, path } => {
                tracing::info!(
                    "Successfully downloaded os icon for {:?} to {:?}",
                    self.config.os_list[index],
                    path
                );
                self.config.os_list[index].icon_local = Some(path);
            }
            BBImagerMessage::OsListDownloadFailed { index, error } => {
                tracing::warn!(
                    "Failed to fetch icon for {:?}, Error: {error}",
                    self.config.imager.devices[index]
                );
            }
            BBImagerMessage::StartFlashing => match self.selected_image.clone().unwrap() {
                OsImage::Local(p) => {
                    return Command::perform(
                        std::future::ready((p, None)),
                        |(path, inner_path)| BBImagerMessage::FlashImage {
                            path,
                            inner_path,
                            sha256: None,
                        },
                    );
                }
                OsImage::Remote(r) => {
                    tracing::info!("Downloading Remote Os");
                    return Command::run(
                        self.downloader.download_progress(r.url, r.download_sha256),
                        |x| BBImagerMessage::DownloadStatus(x.map_err(|y| y.to_string())),
                    );
                }
            },
            BBImagerMessage::DownloadStatus(s) => {
                if let Ok(DownloadStatus::Finished(p)) = s {
                    tracing::info!("Os download finished");
                    self.download_status.take();
                    if let Some(OsImage::Remote(x)) = &self.selected_image {
                        let sha256 = x.extracted_sha256;
                        return Command::perform(
                            std::future::ready((p, x.extract_path.clone())),
                            move |(path, inner_path)| BBImagerMessage::FlashImage {
                                path,
                                inner_path,
                                sha256: Some(sha256),
                            },
                        );
                    } else {
                        unreachable!()
                    }
                } else {
                    tracing::debug!("Os download progress: {:?}", s);
                    self.download_status = Some(s);
                }
            }
            BBImagerMessage::Destinations(x) => match x {
                Ok(y) => self.destinations = y,
                Err(e) => {
                    tracing::error!("Error retriving destinations {e}")
                }
            },
            BBImagerMessage::InitState(s) => {
                self.state = Some(s);
            }
            BBImagerMessage::UnrecoverableError(e) => {
                self.unrecoverable_error = Some(e);
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

    fn home_view(&self) -> Element<BBImagerMessage> {
        const HOME_BTN_PADDING: u16 = 10;

        let logo = widget::image(widget::image::Handle::from_memory(BB_BANNER)).width(500);

        let btn_disable = self.flashing_status.is_some() || self.download_status.is_some();

        let choose_device_btn = match &self.selected_board {
            Some(x) => button(x.name.as_str()),
            None => button("CHOOSE DEVICE"),
        }
        .padding(HOME_BTN_PADDING)
        .on_press_maybe(if btn_disable {
            None
        } else {
            Some(BBImagerMessage::BoardSectionPage)
        });

        let choose_image_btn = match &self.selected_image {
            Some(x) => button(text(x)),
            None => button("CHOOSE IMAGE"),
        }
        .padding(HOME_BTN_PADDING)
        .on_press_maybe(if btn_disable || self.selected_board.is_none() {
            None
        } else {
            Some(BBImagerMessage::ImageSelectionPage)
        });

        let choose_dst_btn = match &self.selected_dst {
            Some(x) => button(x.name.as_str()),
            None => button("CHOOSE DESTINATION"),
        }
        .padding(HOME_BTN_PADDING)
        .on_press_maybe(if btn_disable || self.selected_image.is_none() {
            None
        } else {
            Some(BBImagerMessage::DestinationSelectionPage)
        });

        let reset_btn = button("RESET")
            .padding(HOME_BTN_PADDING)
            .on_press(BBImagerMessage::Reset);

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
            choose_device_btn,
            widget::horizontal_space(),
            choose_image_btn,
            widget::horizontal_space(),
            choose_dst_btn
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
                .on_press(BBImagerMessage::BoardSelected(x.clone()))
                .style(theme::Button::Secondary)
            })
            .map(Into::into);

        let items = widget::scrollable(widget::column(items).spacing(10));

        widget::column![self.search_bar(), widget::horizontal_rule(2), items]
            .spacing(10)
            .padding(10)
            .into()
    }

    fn image_selection_view(&self) -> Element<BBImagerMessage> {
        let board = self.selected_board.as_ref().unwrap();
        let items = self
            .config
            .images_by_device(&board)
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
                .on_press(BBImagerMessage::SelectImage(Some(x.clone())))
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
            self.search_bar(),
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
            self.search_bar(),
            widget::horizontal_rule(2),
            widget::scrollable(widget::column(items).spacing(10))
        ]
        .spacing(10)
        .padding(10)
        .into()
    }

    fn search_bar(&self) -> Element<BBImagerMessage> {
        widget::row![
            button(widget::svg(widget::svg::Handle::from_memory(ARROW_BACK_ICON)).width(22))
                .on_press(BBImagerMessage::HomePage)
                .style(theme::Button::Secondary),
            widget::text_input("Search", &self.search_bar).on_input(BBImagerMessage::Search)
        ]
        .spacing(10)
        .into()
    }

    fn progress(&self) -> (widget::Text, widget::ProgressBar) {
        use std::ops::RangeInclusive;
        use theme::ProgressBar;
        use widget::progress_bar;

        const RANGE: RangeInclusive<f32> = (0.0)..=1.0;

        if let Some(e) = &self.unrecoverable_error {
            (
                text(format!("Unrecoverable Error: {e}")),
                progress_bar(RANGE, 1.0).style(ProgressBar::Danger),
            )
        } else if let Some(s) = &self.download_status {
            match s {
                Ok(x) => match x {
                    DownloadStatus::DownloadingProgress(p) => (
                        text(format!("Downloading... {}%", (*p * 100.0).round() as usize)),
                        progress_bar(RANGE, *p),
                    ),
                    DownloadStatus::Finished(_) => (
                        text("Downloading Successful..."),
                        progress_bar(RANGE, 1.0).style(ProgressBar::Success),
                    ),
                    DownloadStatus::VerifyingProgress(p) => (
                        text(format!("Verifying... {}%", (*p * 100.0).round() as usize)),
                        progress_bar(RANGE, *p),
                    ),
                },
                Err(e) => (
                    text(format!("Downloading Image Failed: {e}")),
                    progress_bar(RANGE, 1.0).style(ProgressBar::Danger),
                ),
            }
        } else if let Some(s) = &self.flashing_status {
            match s {
                Ok(x) => match x {
                    FlashingStatus::Preparing => (text("Preparing..."), progress_bar(RANGE, 0.5)),
                    FlashingStatus::Flashing => (text("Flashing..."), progress_bar(RANGE, 0.5)),
                    FlashingStatus::FlashingProgress(p) => (
                        text(format!("Flashing... {}%", (*p * 100.0).round() as usize)),
                        progress_bar(RANGE, *p),
                    ),
                    FlashingStatus::Verifying => (text("Verifying..."), progress_bar(RANGE, 0.5)),
                    FlashingStatus::VerifyingProgress(p) => (
                        text(format!("Verifying... {}%", (*p * 100.0).round() as usize)),
                        progress_bar(RANGE, *p),
                    ),
                    FlashingStatus::Finished => (
                        text("Flashing Successful..."),
                        progress_bar(RANGE, 1.0).style(ProgressBar::Success),
                    ),
                },
                Err(e) => (
                    text(format!("Flashing Failed: {e}")),
                    progress_bar(RANGE, 1.0).style(ProgressBar::Danger),
                ),
            }
        } else {
            (text(""), widget::progress_bar((0.0)..=1.0, 0.0))
        }
    }
}

#[derive(Default, Debug)]
enum Screen {
    #[default]
    Home,
    BoardSelection,
    ImageSelection,
    DestinationSelection,
}

fn flash_helper(
    path: std::path::PathBuf,
    inner_path: Option<String>,
    sha256: Option<[u8; 32]>,
    board: bb_imager::config::Device,
    dst: bb_imager::Destination,
    state: bb_imager::State,
) -> impl Stream<Item = Result<FlashingStatus, String>> {
    futures_util::stream::once(async move {
        bb_imager::img::OsImage::from_path(&path, inner_path.as_deref(), sha256)
            .await
            .map(|x| board.flasher.flash(x, dst, state))
    })
    .try_flatten()
    .map_err(|x| x.to_string())
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
