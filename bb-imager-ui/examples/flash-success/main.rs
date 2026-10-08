use bb_imager_ui::{Message, flash_success};

struct State {
    inner: flash_success::State,
}

impl State {
    fn new() -> (Self, iced::Task<Message>) {
        let res = Self {
            inner: flash_success::State {
                board: "BeagleY-AI".into(),
                image: "BeagleY-AI Debian 13 v7.1.x-k3 XFCE".into(),
                destination: "Test SD Card".into(),
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
            },
        };

        (res, iced::Task::none())
    }
}

fn main() {
    let app = iced::application(State::new, |_: &mut State, _| iced::Task::none(), view);
    bb_imager_ui::application(app).run().unwrap()
}

fn view(s: &State) -> iced::Element<'_, Message> {
    flash_success::view(&s.inner, iced::widget::Id::unique())
}
