use bb_imager_ui::{Message, flash_cancel};

struct State(flash_cancel::State);

impl State {
    fn new() -> (Self, iced::Task<Message<()>>) {
        let res = State(flash_cancel::State {
            has_customization: true,
        });

        (res, iced::Task::none())
    }
}

fn main() {
    let app = iced::application(State::new, |_: &mut State, _| iced::Task::none(), view);
    bb_imager_ui::application(app).run().unwrap()
}

fn view(s: &State) -> iced::Element<'_, Message<()>> {
    flash_cancel::view(&s.0)
}
