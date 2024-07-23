use std::path::PathBuf;

use iced::{widget::rule::FillMode, Element, Length, Sandbox, Settings};

fn main() -> iced::Result {
    BBImager::run(Settings::default())
}

#[derive(Default, Debug)]
struct BBImager {
    selected_board: Option<BeagleBoardDevice>,
    selected_image: Option<PathBuf>,
    selected_dst: Option<String>,
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

    fn flash(&self, img: PathBuf, port: &str) {
        match self {
            BeagleBoardDevice::BeagleConnectFreedom => bb_imager::bcf::flash(&img, port).unwrap(),
        }
    }

    fn destinations(&self) -> Vec<String> {
        match self {
            BeagleBoardDevice::BeagleConnectFreedom => bb_imager::bcf::possible_devices().unwrap(),
        }
    }
}

impl Sandbox for BBImager {
    type Message = BBImagerMessage;

    fn new() -> Self {
        Self::default()
    }

    fn title(&self) -> String {
        String::from("BeagleBoard Imager")
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            BBImagerMessage::BoardSelected(x) => {
                self.selected_board = Some(x);
            }
            BBImagerMessage::SelectImage => {
                self.selected_image = rfd::FileDialog::new()
                    .add_filter("firmware", &["bin"])
                    .pick_file()
            }
            BBImagerMessage::SelectPort(x) => self.selected_dst = Some(x),
            BBImagerMessage::FlashImage { board, img, port } => board.flash(img, &port),
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

        iced::widget::column(items)
            .align_items(iced::Alignment::Center)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(10)
            .spacing(20)
            .into()
    }
}
