use bb_imager_ui::{Message, flash_cancel};

struct State;

impl State {
    fn new() -> (Self, iced::Task<Message>) {
        let res = Self;

        (res, iced::Task::none())
    }
}

fn main() {
    let app = iced::application(State::new, |_: &mut State, _| iced::Task::none(), view);
    bb_imager_ui::application(app).run().unwrap()
}

fn view(_s: &State) -> iced::Element<'_, Message> {
    flash_cancel::view(iced::widget::Id::unique())
}
