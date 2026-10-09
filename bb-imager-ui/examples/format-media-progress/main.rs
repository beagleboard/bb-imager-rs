use bb_imager_ui::{Message, format_media_progress};

struct State(format_media_progress::State);

impl State {
    fn new() -> (Self, iced::Task<Message>) {
        let res = State(format_media_progress::State {
            destination: "Test SD Card (32 GB)".into(),
        });

        (res, iced::Task::none())
    }
}

fn main() {
    let app = iced::application(State::new, |_: &mut State, _| iced::Task::none(), view);
    bb_imager_ui::application(app).run().unwrap()
}

fn view(s: &State) -> iced::Element<'_, Message> {
    format_media_progress::view(&s.0)
}
