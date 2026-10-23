use bb_imager_ui::{Message, review};

struct State(review::State);

impl State {
    fn new() -> (Self, iced::Task<Message>) {
        let res = State(review::State {
            is_download: true,
            board: "BeagleY-AI".into(),
            image: "BeagleY-AI Debian 13 v7.1.x-k3 XFCE".into(),
            destinations: ["Test SD Card".into()].into(),
            modifications: vec![
                "User account configured",
                "Wifi configured",
                "Hostname configured",
                "Keymap configured",
                "Timezone configured",
                "SSH Key configured",
                "USB DHCP enabled",
            ]
            .into(),
            has_customization: true,
        });

        (res, iced::Task::none())
    }
}

fn main() {
    let app = iced::application(State::new, |_: &mut State, _| iced::Task::none(), view);
    bb_imager_ui::application(app).run().unwrap()
}

fn view(s: &State) -> iced::Element<'_, Message> {
    review::view(&s.0, iced::widget::Id::unique())
}
