use std::path::PathBuf;

use iced::{
    executor,
    futures::{SinkExt, Stream, StreamExt},
    Application, Command, Element, Settings,
};

fn main() -> iced::Result {
    tracing_subscriber::fmt().init();

    let mut settings = Settings::default();

    settings.window.size = iced::Size::new(700.0, 500.0);

    BBImager::run(settings)
}

#[derive(Default, Debug)]
struct BBImager {
    selected_board: Option<BeagleBoardDevice>,
    selected_image: Option<PathBuf>,
    selected_dst: Option<String>,
    flashing_status: Option<Result<bb_imager::Status, String>>,
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
                Command::none()
            }
            BBImagerMessage::SelectImage => {
                self.selected_image = rfd::FileDialog::new()
                    .add_filter("firmware", &["bin"])
                    .pick_file();
                Command::none()
            }
            BBImagerMessage::SelectPort(x) => {
                self.selected_dst = Some(x);
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
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<Self::Message> {
        const BTN_PADDING: u16 = 10;

        let logo = iced::widget::image("icons/logo_sxs_imager.png").width(500);

        let choose_device_btn = iced::widget::button("CHOOSE DEVICE").padding(BTN_PADDING);
        let choose_image_btn = iced::widget::button("CHOOSE IMAGE").padding(BTN_PADDING);
        let choose_dst_btn = iced::widget::button("CHOOSE DESTINATION").padding(BTN_PADDING);

        let reset_btn = iced::widget::button("RESET")
            .on_press(BBImagerMessage::Reset)
            .padding(BTN_PADDING);
        let write_btn = iced::widget::button("WRITE").padding(BTN_PADDING);

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

        iced::widget::column![logo, choice_btn_row, action_btn_row]
            .padding(64)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .align_items(iced::Alignment::Center)
            .into()
    }

    fn theme(&self) -> Self::Theme {
        iced::Theme::KanagawaLotus
    }
}
