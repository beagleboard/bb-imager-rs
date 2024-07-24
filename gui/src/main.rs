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
        ..Default::default()
    };

    BBImager::run(settings)
}

#[derive(Default, Debug)]
struct BBImager {
    screen: Screen,
    selected_board: Option<BeagleBoardDevice>,
    selected_image: Option<PathBuf>,
    selected_dst: Option<String>,
    flashing_status: Option<Result<bb_imager::Status, String>>,
    search_bar: String,
}

#[derive(Debug, Clone)]
enum BBImagerMessage {
    BoardSelected(BeagleBoardDevice),
    SelectImage,
    SelectPort(String),
    FlashImage {
        board: BeagleBoardDevice,
        img: PathBuf,
        port: String,
    },

    FlashingStatus(bb_imager::Status),
    FlashingFail(String),
    Reset,

    BoardSectionPage,
    ImageSelectionPage,
    DestinationSelectionPage,
    HomePage,

    Search(String),
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum BeagleBoardDevice {
    BeagleConnectFreedom,
}

impl std::fmt::Display for BeagleBoardDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BeagleBoardDevice::BeagleConnectFreedom => write!(f, "BeagleConnect Freedom"),
        }
    }
}

impl BeagleBoardDevice {
    const ALL: &[Self] = &[Self::BeagleConnectFreedom];

    fn flash(
        &self,
        img: PathBuf,
        port: String,
    ) -> impl Stream<Item = Result<bb_imager::Status, bb_imager::bcf::BeagleConnectFreedomError>>
    {
        match self {
            BeagleBoardDevice::BeagleConnectFreedom => bb_imager::bcf::flash(img, port),
        }
    }

    fn destinations(&self) -> Vec<String> {
        match self {
            BeagleBoardDevice::BeagleConnectFreedom => bb_imager::bcf::possible_devices().unwrap(),
        }
    }
}

impl Application for BBImager {
    type Message = BBImagerMessage;
    type Executor = executor::Default;
    type Flags = ();
    type Theme = iced::theme::Theme;

    fn new(_flags: ()) -> (Self, Command<Self::Message>) {
        (Self::default(), Command::none())
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
            BBImagerMessage::FlashImage { board, img, port } => {
                iced::command::channel(10, move |mut tx| async move {
                    let stream = board.flash(img, port);

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
                .map_or(iced::widget::text("CHOOSE DEVICE"), |x| {
                    iced::widget::text(x.to_string())
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
        let write_btn = if let (Some(board), Some(img), Some(port)) = (
            self.selected_board,
            self.selected_image.clone(),
            self.selected_dst.clone(),
        ) {
            iced::widget::button("WRITE").on_press(BBImagerMessage::FlashImage { board, img, port })
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
        let item = iced::widget::button(
            iced::widget::row![
                iced::widget::image("icons/bcf.webp").width(100),
                iced::widget::column![
                    iced::widget::text("BeagleConnect Freedom").size(18),
                    iced::widget::horizontal_space(),
                    iced::widget::text("BeagleConnect Freedom based on Ti CC1352P7")
                ]
                .padding(5)
            ]
            .spacing(10),
        )
        .width(iced::Length::Fill)
        .on_press(BBImagerMessage::BoardSelected(
            BeagleBoardDevice::BeagleConnectFreedom,
        ))
        .style(iced::widget::theme::Button::Secondary);

        let items = iced::widget::scrollable(iced::widget::column![item].spacing(10));

        iced::widget::column![self.search_bar(), iced::widget::horizontal_rule(2), items]
            .spacing(10)
            .padding(10)
            .into()
    }

    fn image_selection_view(&self) -> Element<BBImagerMessage> {
        let items = [(
            PathBuf::from("icons/use_custom.png"),
            "Use Custom Image".to_string(),
            BBImagerMessage::SelectImage,
        )];

        iced::widget::column![
            self.search_bar(),
            iced::widget::horizontal_rule(2),
            self.search_list_img(items)
        ]
        .spacing(10)
        .padding(10)
        .into()
    }

    fn destination_selection_view(&self) -> Element<BBImagerMessage> {
        let items = self
            .selected_board
            .expect("No Board Selected")
            .destinations()
            .into_iter()
            .map(|x| {
                (
                    PathBuf::from("icons/ic_usb_40px.svg"),
                    x.clone(),
                    BBImagerMessage::SelectPort(x),
                )
            });

        iced::widget::column![
            self.search_bar(),
            iced::widget::horizontal_rule(2),
            self.search_list_svg(items)
        ]
        .spacing(10)
        .padding(10)
        .into()
    }

    fn search_list_img(
        &self,
        items: impl IntoIterator<Item = (PathBuf, String, BBImagerMessage)>,
    ) -> Element<BBImagerMessage> {
        let items = items
            .into_iter()
            .filter(|(_, x, _)| x.to_lowercase().contains(&self.search_bar.to_lowercase()))
            .map(|(p, t, o)| {
                iced::widget::button(iced::widget::row![
                    iced::widget::image(p).width(100),
                    iced::widget::text(t)
                ])
                .width(iced::Length::Fill)
                .on_press(o)
                .style(iced::widget::theme::Button::Secondary)
            })
            .map(Into::into);

        iced::widget::scrollable(iced::widget::column(items).spacing(10)).into()
    }

    fn search_list_svg(
        &self,
        items: impl IntoIterator<Item = (PathBuf, String, BBImagerMessage)>,
    ) -> Element<BBImagerMessage> {
        let items = items
            .into_iter()
            .filter(|(_, x, _)| x.to_lowercase().contains(&self.search_bar.to_lowercase()))
            .map(|(p, t, o)| {
                iced::widget::button(iced::widget::row![
                    iced::widget::svg(p).width(100),
                    iced::widget::text(t)
                ])
                .width(iced::Length::Fill)
                .on_press(o)
                .style(iced::widget::theme::Button::Secondary)
            })
            .map(Into::into);

        iced::widget::scrollable(iced::widget::column(items).spacing(10)).into()
    }

    fn search_bar(&self) -> Element<BBImagerMessage> {
        iced::widget::row![
            iced::widget::button("Back")
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
