use bb_imager_ui::{Message, sandbox_notice};

struct State;

impl State {
    fn new() -> (Self, iced::Task<Message>) {
        (State, iced::Task::none())
    }
}

fn main() {
    let app = iced::application(State::new, |_: &mut State, _| iced::Task::none(), view);
    bb_imager_ui::application(app).run().unwrap()
}

fn view(_: &State) -> iced::Element<'_, Message> {
    sandbox_notice::view(iced::widget::Id::unique())
}