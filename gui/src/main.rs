use iced::{widget::rule::FillMode, Element, Length, Sandbox, Settings};

fn main() -> iced::Result {
    BBImager::run(Settings::default())
}

struct BBImager {
    selected_board: Option<BeagleBoardDevice>,
}

#[derive(Debug, Clone)]
enum BBImagerMessage {
    BoardSelected(BeagleBoardDevice),
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
}

impl Sandbox for BBImager {
    type Message = BBImagerMessage;

    fn new() -> Self {
        Self {
            selected_board: None,
        }
    }

    fn title(&self) -> String {
        String::from("BeagleBoard Imager")
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            BBImagerMessage::BoardSelected(x) => self.selected_board = Some(x),
        }
    }

    fn view(&self) -> Element<Self::Message> {
        let device_list =
            iced::widget::pick_list(BeagleBoardDevice::ALL, self.selected_board, |x| {
                BBImagerMessage::BoardSelected(x)
            })
            .placeholder("Choose Device");
        iced::widget::row![device_list]
            .align_items(iced::Alignment::Center)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(10)
            .into()
    }
}
