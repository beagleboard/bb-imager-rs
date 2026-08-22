use bb_imager_ui::{Message, dest_selection};

#[derive(Clone, Debug, Default)]
struct Destination(u64);

impl std::fmt::Display for Destination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Test SD Card {}", self.0)
    }
}

impl dest_selection::Destination for Destination {
    fn size(&self) -> Option<u64> {
        Some(self.0 * 1024 * 1024 * 1024)
    }
}

struct State(dest_selection::State<Destination>);

impl State {
    fn new() -> (Self, iced::Task<Message<Destination>>) {
        let res = dest_selection::State {
            destinations: (1..15).map(Destination).collect(),
            search: "".into(),
            image_file_name: Some("temp.img".into()),
            filter_destination: true,
            instructions: "1. Connect the BeagleV-Fire board to your computer via USB.\n2. While powering on the board, click the USER button on the board as soon as you power it on.\n3. BeagleV-Fire must appear as a USB device in the destination list.".into()
        };

        (Self(res), iced::Task::none())
    }
}

fn main() {
    let app = iced::application(
        State::new,
        |s: &mut State, msg| {
            match msg {
                Message::UpdateSearchText(search) => {
                    s.0.search = search;
                }
                Message::DestinationFilter(x) => {
                    s.0.filter_destination = x;
                }
                _ => {}
            };
            iced::Task::none()
        },
        view,
    );

    bb_imager_ui::application(app).run().unwrap()
}

fn view(s: &State) -> iced::Element<'_, Message<Destination>> {
    dest_selection::view(&s.0, iced::widget::Id::unique())
}
