use std::path::PathBuf;

use iced::{
    executor,
    futures::{SinkExt, Stream, StreamExt},
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
                self.screen = Screen::Home;
                Command::none()
            }
            BBImagerMessage::SelectImage => {
                self.selected_image = rfd::FileDialog::new()
                    .add_filter("firmware", &["bin"])
                    .pick_file();
                self.screen = Screen::Home;
                Command::none()
            }
            BBImagerMessage::SelectPort(x) => {
                self.selected_dst = Some(x);
                self.screen = Screen::Home;
                Command::none()
            }
            BBImagerMessage::FlashImage => {
                let board = self.selected_board.clone().expect("No board selected");
                let img = self.selected_image.clone().expect("No image selected");
                let dst = self.selected_dst.clone().expect("No destination selected");

                iced::command::channel(10, move |mut tx| async move {
                    let stream = board.flasher.flash(img, dst);

                    tokio::pin!(stream);

                    while let Some(x) = stream.next().await {
                        let temp = match x {
                            Ok(y) => BBImagerMessage::FlashingStatus(y),
                            Err(y) => BBImagerMessage::FlashingFail(y.to_string()),
                        };

                        let _ = tx.send(temp).await;
                    }
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
                self.screen = Screen::Home;
                self.search_bar.clear();
                Command::none()
            }
            BBImagerMessage::BoardSectionPage => {
                self.screen = Screen::BoardSelection;
                Command::none()
            }
            BBImagerMessage::ImageSelectionPage => {
                self.screen = Screen::ImageSelection;
                Command::none()
            }
            BBImagerMessage::DestinationSelectionPage => {
                self.screen = Screen::DestinationSelection;
                Command::none()
            }

            BBImagerMessage::Search(x) => {
                self.search_bar = x;
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
    fn home_view(&self) -> Element<BBImagerMessage> {
        const BTN_PADDING: u16 = 10;

        let logo = iced::widget::image("icons/logo_sxs_imager.png").width(500);

        let choose_device_btn = iced::widget::button(
            self.selected_board
                .clone()
                .map_or(iced::widget::text("CHOOSE DEVICE"), |x| {
                    iced::widget::text(x.name)
                }),
        )
        .on_press(BBImagerMessage::BoardSectionPage)
        .padding(BTN_PADDING);

        let choose_image_btn = iced::widget::button(
            self.selected_image
                .clone()
                .map_or(iced::widget::text("CHOOSE IMAGE"), |x| {
                    iced::widget::text(x.file_name().unwrap().to_string_lossy())
                }),
        )
        .on_press_maybe(
            self.selected_board
                .clone()
                .map(|_| BBImagerMessage::ImageSelectionPage),
        )
        .padding(BTN_PADDING);

        let choose_dst_btn = iced::widget::button(
            self.selected_dst
                .clone()
                .map_or(iced::widget::text("CHOOSE DESTINATION"), |x| {
                    iced::widget::text(x)
                }),
        )
        .on_press_maybe(
            self.selected_image
                .clone()
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
            .map(|x| {
                iced::widget::button(
                    iced::widget::row![
                        iced::widget::image(x.icon.to_string()).width(100),
                        iced::widget::column![
                            iced::widget::text(x.name.clone()).size(18),
                            iced::widget::horizontal_space(),
                            iced::widget::text(x.description.clone())
                        ]
                        .padding(5)
                    ]
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
        let board = self.selected_board.clone().unwrap();
        let items = self
            .config
            .images_by_device(&board)
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
                        iced::widget::image(x.icon.to_string()).width(100),
                        iced::widget::column![
                            iced::widget::text(x.name.clone()).size(18),
                            iced::widget::text(x.description.clone()),
                            row3
                        ]
                        .padding(5)
                    ]
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
            .clone()
            .expect("No Board Selected")
            .flasher
            .destinations()
            .unwrap()
            .into_iter()
            .map(|x| {
                (
                    PathBuf::from("icons/usb.svg"),
                    x.clone(),
                    BBImagerMessage::SelectPort(x),
                )
            });

        iced::widget::column![
            self.search_bar(),
            iced::widget::horizontal_rule(2),
            self.search_list(items)
        ]
        .spacing(10)
        .padding(10)
        .into()
    }

    fn search_list(
        &self,
        items: impl IntoIterator<Item = (PathBuf, String, BBImagerMessage)>,
    ) -> Element<BBImagerMessage> {
        let items = items
            .into_iter()
            .filter(|(_, x, _)| x.to_lowercase().contains(&self.search_bar.to_lowercase()))
            .map(|(p, t, o)| {
                iced::widget::button(
                    iced::widget::row![iced::widget::svg(p).width(40), iced::widget::text(t),]
                        .align_items(iced::Alignment::Center)
                        .spacing(10),
                )
                .width(iced::Length::Fill)
                .on_press(o)
                .style(iced::widget::theme::Button::Secondary)
            })
            .map(Into::into);

        iced::widget::scrollable(iced::widget::column(items).spacing(10)).into()
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
