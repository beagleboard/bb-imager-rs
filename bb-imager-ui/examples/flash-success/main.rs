use bb_imager_ui::{Message, flash_success};

struct State(flash_success::State);

impl State {
    fn new() -> (Self, iced::Task<Message<()>>) {
        let res = State(flash_success::State {
            has_customization: true,
            device_name: "BeagleY-AI".into(),
            software_name: "BeagleY-AI Debian 13 v7.1.x-k3 XFCE".into(),
            storage: ("Test SD Card".into(), Some(4 * 1024 * 1024 * 1024)),
            modificiations: vec![
                "User account configured",
                "Wifi configured",
                "Hostname configured",
                "Keymap configured",
                "Timezone configured",
                "SSH Key configured",
                "USB DHCP enabled",
            ],
        });

        (res, iced::Task::none())
    }
}

fn main() {
    let app = iced::application(
        State::new,
        |_: &mut State, msg| match msg {
            Message::CopyToClipboard => {
                iced::clipboard::write("All info regarding board, image and destination".into())
            }
            _ => iced::Task::none(),
        },
        view,
    );
    bb_imager_ui::application(app).run().unwrap()
}

fn view(s: &State) -> iced::Element<'_, Message<()>> {
    flash_success::view(&s.0, iced::widget::Id::unique())
}
