use std::{
    future::Future,
    path::{Path, PathBuf},
    time::Duration,
};

use iced::{
    advanced::graphics::futures::MaybeSend,
    executor,
    futures::{SinkExt, StreamExt},
    Application, Command, Element, Settings,
};

fn main() -> iced::Result {
    tracing_subscriber::fmt().init();

    let settings = Settings {
        window: iced::window::Settings {
            size: iced::Size::new(800.0, 500.0),
            icon: iced::window::icon::from_file("icons/bb-imager.png").ok(),
            ..Default::default()
        },
        flags: bb_imager::config::Config::from_json(include_bytes!("../../config.json"))
            .expect("Failed to parse config"),
        ..Default::default()
    };

    BBImager::run(settings)
}

#[derive(Default, Debug)]
struct BBImager {
    config: bb_imager::config::Config,
    screen: Screen,
    selected_board: Option<bb_imager::config::Device>,
    selected_image: Option<PathBuf>,
    selected_dst: Option<String>,
    flashing_status: Option<Result<bb_imager::Status, String>>,
    search_bar: String,
}

#[derive(Debug, Clone)]
enum BBImagerMessage {
    BoardSelected(bb_imager::config::Device),
    SelectImage,
    SelectPort(String),
    FlashImage,

    FlashingStatus(bb_imager::Status),
    FlashingFail(String),
    Reset,

    BoardSectionPage,
    ImageSelectionPage,
    DestinationSelectionPage,
    HomePage,

    Search(String),
    BoardImageDownloaded { index: usize, path: PathBuf },
    BoardImageDownloadFailed { index: usize, error: String },

    OsListImageDownloaded { index: usize, path: PathBuf },
    OsListDownloadFailed { index: usize, error: String },
}

impl Application for BBImager {
    type Message = BBImagerMessage;
    type Executor = executor::Default;
    type Flags = bb_imager::config::Config;
    type Theme = iced::theme::Theme;

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Self {
                config: flags,
                ..Default::default()
            },
            Command::none(),
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
                Command::none()
            }
            BBImagerMessage::SelectImage => {
                self.selected_image = rfd::FileDialog::new()
                    .add_filter("firmware", &["bin"])
                    .pick_file();
                self.back_home();
                Command::none()
            }
            BBImagerMessage::SelectPort(x) => {
                self.selected_dst = Some(x);
                self.back_home();
                Command::none()
            }
            BBImagerMessage::FlashImage => {
                let board = self.selected_board.clone().expect("No board selected");
                let img = self.selected_image.clone().expect("No image selected");
                let dst = self.selected_dst.clone().expect("No destination selected");

                Command::run(board.flasher.flash(img, dst), |x| match x {
                    Ok(y) => BBImagerMessage::FlashingStatus(y),
                    Err(y) => BBImagerMessage::FlashingFail(y.to_string()),
                })
            }
            BBImagerMessage::FlashingStatus(x) => {
                self.flashing_status = Some(Ok(x));
                Command::none()
            }
            BBImagerMessage::FlashingFail(x) => {
                self.flashing_status = Some(Err(x));
                Command::none()
            }
            BBImagerMessage::Reset => {
                self.selected_dst = None;
                self.selected_image = None;
                self.selected_board = None;
                self.search_bar.clear();
                Command::none()
            }
            BBImagerMessage::HomePage => {
                self.back_home();
                Command::none()
            }
            BBImagerMessage::BoardSectionPage => {
                self.screen = Screen::BoardSelection;
                tracing::info!("Start Image Download");
                Command::batch(self.config.devices().iter().enumerate().map(|(index, v)| {
                    Command::perform(
                        image_resolver(v.icon.to_string(), v.icon_sha256.clone()),
                        move |p| match p {
                            Ok(path) => BBImagerMessage::BoardImageDownloaded { index, path },
                            Err(e) => BBImagerMessage::BoardImageDownloadFailed {
                                index,
                                error: e.to_string(),
                            },
                        },
                    )
                }))
            }
            BBImagerMessage::ImageSelectionPage => {
                self.screen = Screen::ImageSelection;
                let board = self.selected_board.as_ref().unwrap().name.clone();

                Command::batch(
                    self.config
                        .os_list
                        .iter()
                        .enumerate()
                        .filter(|(_, v)| v.devices.contains(&board))
                        .map(|(index, v)| {
                            Command::perform(
                                image_resolver(v.icon.to_string(), v.icon_sha256.clone()),
                                move |p| match p {
                                    Ok(path) => {
                                        BBImagerMessage::OsListImageDownloaded { index, path }
                                    }
                                    Err(e) => BBImagerMessage::OsListDownloadFailed {
                                        index,
                                        error: e.to_string(),
                                    },
                                },
                            )
                        }),
                )
            }
            BBImagerMessage::DestinationSelectionPage => {
                self.screen = Screen::DestinationSelection;
                Command::none()
            }

            BBImagerMessage::Search(x) => {
                self.search_bar = x;
                Command::none()
            }
            BBImagerMessage::BoardImageDownloaded { index, path } => {
                tracing::info!("Successfully downloaded to {:?}", path);
                self.config.imager.devices[index].icon_local = Some(path);
                Command::none()
            }
            BBImagerMessage::BoardImageDownloadFailed { index, error } => {
                tracing::warn!(
                    "Failed to fetch icon for {:?}, Error: {error}",
                    self.config.imager.devices[index]
                );
                Command::none()
            }
            BBImagerMessage::OsListImageDownloaded { index, path } => {
                tracing::info!("Successfully downloaded to {:?}", path);
                self.config.os_list[index].icon_local = Some(path);
                Command::none()
            }
            BBImagerMessage::OsListDownloadFailed { index, error } => {
                tracing::warn!(
                    "Failed to fetch icon for {:?}, Error: {error}",
                    self.config.imager.devices[index]
                );
                Command::none()
            }
        }
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
        const BTN_PADDING: u16 = 10;

        let logo = iced::widget::image("icons/logo_sxs_imager.png").width(500);

        let choose_device_btn = iced::widget::button(
            self.selected_board
                .as_ref()
                .map_or(iced::widget::text("CHOOSE DEVICE"), |x| {
                    iced::widget::text(x.name.as_str())
                }),
        )
        .on_press(BBImagerMessage::BoardSectionPage)
        .padding(BTN_PADDING);

        let choose_image_btn = iced::widget::button(
            self.selected_image
                .as_ref()
                .map_or(iced::widget::text("CHOOSE IMAGE"), |x| {
                    iced::widget::text(x.file_name().unwrap().to_string_lossy())
                }),
        )
        .on_press_maybe(
            self.selected_board
                .as_ref()
                .map(|_| BBImagerMessage::ImageSelectionPage),
        )
        .padding(BTN_PADDING);

        let choose_dst_btn = iced::widget::button(
            self.selected_dst
                .as_ref()
                .map_or(iced::widget::text("CHOOSE DESTINATION"), iced::widget::text),
        )
        .on_press_maybe(
            self.selected_image
                .as_ref()
                .map(|_| BBImagerMessage::DestinationSelectionPage),
        )
        .padding(BTN_PADDING);

        let reset_btn = iced::widget::button("RESET")
            .on_press(BBImagerMessage::Reset)
            .padding(BTN_PADDING);
        let write_btn = if self.selected_board.is_some()
            && self.selected_image.is_some()
            && self.selected_dst.is_some()
        {
            iced::widget::button("WRITE").on_press(BBImagerMessage::FlashImage)
        } else {
            iced::widget::button("WRITE")
        }
        .padding(BTN_PADDING);

        let choice_btn_row = iced::widget::row![
            choose_device_btn,
            iced::widget::horizontal_space(),
            choose_image_btn,
            iced::widget::horizontal_space(),
            choose_dst_btn
        ]
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .align_items(iced::Alignment::Center);

        let action_btn_row =
            iced::widget::row![reset_btn, iced::widget::horizontal_space(), write_btn]
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .align_items(iced::Alignment::Center);

        let (progress_label, progress_bar) = match &self.flashing_status {
            Some(x) => match x {
                Ok(y) => match y {
                    bb_imager::Status::Preparing => (
                        iced::widget::text("Preparing..."),
                        iced::widget::progress_bar((0.0)..=1.0, 0.0),
                    ),
                    bb_imager::Status::Flashing => (
                        iced::widget::text("Flashing Image..."),
                        iced::widget::progress_bar((0.0)..=1.0, 0.0),
                    ),
                    bb_imager::Status::FlashingProgress(p) => (
                        iced::widget::text("Flashing Image..."),
                        iced::widget::progress_bar((0.0)..=1.0, *p),
                    ),
                    bb_imager::Status::Verifying => (
                        iced::widget::text("Verifying Image..."),
                        iced::widget::progress_bar((0.0)..=1.0, 0.5),
                    ),
                    bb_imager::Status::VerifyingProgress(p) => (
                        iced::widget::text("Verifying Image..."),
                        iced::widget::progress_bar((0.0)..=1.0, *p),
                    ),
                    bb_imager::Status::Finished => (
                        iced::widget::text(format!("Flashing Success!!")),
                        iced::widget::progress_bar((0.0)..=1.0, 1.0)
                            .style(iced::widget::theme::ProgressBar::Success),
                    ),
                },
                Err(e) => (
                    iced::widget::text(format!("Flashing Failed: {e}")),
                    iced::widget::progress_bar((0.0)..=1.0, 1.0)
                        .style(iced::widget::theme::ProgressBar::Danger),
                ),
            },
            None => (
                iced::widget::text(""),
                iced::widget::progress_bar((0.0)..=1.0, 0.0),
            ),
        };

        iced::widget::column![
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
                    Some(y) => iced::widget::image(iced::widget::image::Handle::from_memory(
                        std::fs::read(y).unwrap(),
                    ))
                    .width(100)
                    .height(100)
                    .into(),
                    None => iced::widget::svg("icons/downloading.svg").width(40).into(),
                };

                iced::widget::button(
                    iced::widget::row![
                        image,
                        iced::widget::column![
                            iced::widget::text(x.name.as_str()).size(18),
                            iced::widget::horizontal_space(),
                            iced::widget::text(x.description.as_str())
                        ]
                        .padding(5)
                    ]
                    .align_items(iced::Alignment::Center)
                    .spacing(10),
                )
                .width(iced::Length::Fill)
                .on_press(BBImagerMessage::BoardSelected(x.clone()))
                .style(iced::widget::theme::Button::Secondary)
            })
            .map(Into::into);

        let items = iced::widget::scrollable(iced::widget::column(items).spacing(10));

        iced::widget::column![self.search_bar(), iced::widget::horizontal_rule(2), items]
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
                let mut row3 = iced::widget::row![
                    iced::widget::text(x.version.clone()),
                    iced::widget::horizontal_space(),
                ]
                .width(iced::Length::Fill);

                row3 = x.tags.iter().fold(row3, |acc, t| {
                    acc.push(iced_aw::Badge::new(iced::widget::text(t)))
                });

                row3 = row3.push(iced::widget::horizontal_space());
                row3 = row3.push(iced::widget::text(x.release_date));

                iced::widget::button(
                    iced::widget::row![
                        iced::widget::svg(
                            x.icon_local
                                .clone()
                                .unwrap_or(PathBuf::from("icons/downloading.svg"))
                        )
                        .width(100),
                        iced::widget::column![
                            iced::widget::text(x.name.as_str()).size(18),
                            iced::widget::text(x.description.as_str()),
                            row3
                        ]
                        .padding(5)
                    ]
                    .align_items(iced::Alignment::Center)
                    .spacing(10),
                )
                .width(iced::Length::Fill)
                .on_press(BBImagerMessage::SelectImage)
                .style(iced::widget::theme::Button::Secondary)
            })
            .chain(std::iter::once(
                iced::widget::button(
                    iced::widget::row![
                        iced::widget::svg("icons/file-add.svg").width(100),
                        iced::widget::text("Use Custom Image").size(18),
                    ]
                    .spacing(10),
                )
                .width(iced::Length::Fill)
                .on_press(BBImagerMessage::SelectImage)
                .style(iced::widget::theme::Button::Secondary),
            ))
            .map(Into::into);

        iced::widget::column![
            self.search_bar(),
            iced::widget::horizontal_rule(2),
            iced::widget::scrollable(iced::widget::column(items).spacing(10))
        ]
        .spacing(10)
        .padding(10)
        .into()
    }

    fn destination_selection_view(&self) -> Element<BBImagerMessage> {
        let items = self
            .selected_board
            .as_ref()
            .expect("No Board Selected")
            .flasher
            .destinations()
            .unwrap()
            .into_iter()
            .filter(|x| x.to_lowercase().contains(&self.search_bar.to_lowercase()))
            .map(|x| {
                iced::widget::button(
                    iced::widget::row![
                        iced::widget::svg("icons/usb.svg").width(40),
                        iced::widget::text(x.as_str()),
                    ]
                    .align_items(iced::Alignment::Center)
                    .spacing(10),
                )
                .width(iced::Length::Fill)
                .on_press(BBImagerMessage::SelectPort(x))
                .style(iced::widget::theme::Button::Secondary)
            })
            .map(Into::into);

        iced::widget::column![
            self.search_bar(),
            iced::widget::horizontal_rule(2),
            iced::widget::scrollable(iced::widget::column(items).spacing(10))
        ]
        .spacing(10)
        .padding(10)
        .into()
    }

    fn search_bar(&self) -> Element<BBImagerMessage> {
        iced::widget::row![
            iced::widget::button(iced::widget::svg("icons/arrow-back.svg").width(22))
                .on_press(BBImagerMessage::HomePage)
                .style(iced::widget::theme::Button::Secondary),
            iced::widget::text_input("Search", &self.search_bar).on_input(BBImagerMessage::Search)
        ]
        .spacing(10)
        .into()
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

async fn image_resolver(icon: String, sha256: Vec<u8>) -> Result<PathBuf, data_downloader::Error> {
    tokio::task::spawn_blocking(move || {
        let downloader = data_downloader::DownloadRequest {
            url: icon.as_str(),
            sha256_hash: sha256.as_slice(),
        };

        data_downloader::DownloaderBuilder::new()
            .retry_attempts(0)
            .timeout(Some(Duration::from_secs(10)))
            .build()
            .unwrap()
            .get_path(&downloader)
    })
    .await
    .unwrap()
}

mod remote_image {
    use super::BBImagerMessage;

    pub struct RemoteImage {
        url: String,
        sha256: Vec<u8>,
    }

    impl RemoteImage {
        pub fn new(url: String, sha256: Vec<u8>) -> Self {
            Self { url, sha256 }
        }
    }

    impl iced::widget::Component<BBImagerMessage> for RemoteImage {
        type State = bool;

        type Event = ();

        fn update(
            &mut self,
            state: &mut Self::State,
            _event: Self::Event,
        ) -> Option<BBImagerMessage> {
            None
        }

        fn view(
            &self,
            _state: &Self::State,
        ) -> iced::Element<'_, Self::Event, iced::Theme, iced::Renderer> {
            let img = data_downloader::DownloadRequest {
                url: &self.url,
                sha256_hash: &self.sha256,
            };

            match data_downloader::get_cached(&img) {
                Ok(x) => iced::widget::image(iced::widget::image::Handle::from_memory(x))
                    .width(40)
                    .into(),
                Err(_) => iced::widget::svg("icons/downloading.svg").width(40).into(),
            }
        }
    }
}
