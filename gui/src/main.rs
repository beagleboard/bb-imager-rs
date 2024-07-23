use std::{ops::RangeInclusive, path::PathBuf};

use iced::{
    executor,
    futures::{SinkExt, Stream, StreamExt},
    Application, Command, Element, Length, Settings, Theme,
};

fn main() -> iced::Result {
    tracing_subscriber::fmt().init();

    BBImager::run(Settings::default())
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
    type Theme = Theme;

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
        }
    }

    fn view(&self) -> Element<Self::Message> {
        let device_list =
            iced::widget::pick_list(BeagleBoardDevice::ALL, self.selected_board, |x| {
                BBImagerMessage::BoardSelected(x)
            })
            .placeholder("Choose Device");

        let mut items = Vec::from([device_list.into()]);

        if self.selected_board.is_some() {
            let file = self
                .selected_image
                .clone()
                .map_or(iced::widget::text("Choose Image"), |x| {
                    iced::widget::text(x.to_string_lossy())
                });
            items.push(
                iced::widget::button(file)
                    .on_press(BBImagerMessage::SelectImage)
                    .into(),
            );
        }

        if self.selected_image.is_some() {
            if let Some(x) = self.selected_board {
                let destinations = x.destinations();
                items.push(
                    iced::widget::pick_list(destinations, self.selected_dst.clone(), |x| {
                        BBImagerMessage::SelectPort(x)
                    })
                    .into(),
                )
            }
        }

        if let (Some(board), Some(img), Some(port)) = (
            self.selected_board,
            self.selected_image.clone(),
            self.selected_dst.clone(),
        ) {
            items.push(
                iced::widget::button("Flash")
                    .on_press(BBImagerMessage::FlashImage { board, img, port })
                    .into(),
            )
        }

        if let Some(status) = &self.flashing_status {
            match status {
                Ok(x) => match x {
                    bb_imager::Status::Preparing => {
                        items.push(iced::widget::text("Preparing").into());
                        items.push(
                            iced::widget::progress_bar(RangeInclusive::new(0.0, 1.0), 0.0).into(),
                        );
                    }
                    bb_imager::Status::Flashing(x) => {
                        items.push(iced::widget::text("Flashing").into());
                        items.push(
                            iced::widget::progress_bar(RangeInclusive::new(0.0, 1.0), *x).into(),
                        );
                    }
                    bb_imager::Status::Verifying => {
                        items.push(iced::widget::text("Verifying").into());
                        items.push(
                            iced::widget::progress_bar(RangeInclusive::new(0.0, 1.0), 0.0).into(),
                        );
                    }
                    bb_imager::Status::Finished => {
                        items.push(iced::widget::text("Flashed Successfull").into());
                    }
                },
                Err(x) => {
                    items.push(iced::widget::text(format!("Flashed failed: {x}")).into());
                }
            }
        }

        iced::widget::column(items)
            .align_items(iced::Alignment::Center)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(10)
            .spacing(20)
            .into()
    }
}
