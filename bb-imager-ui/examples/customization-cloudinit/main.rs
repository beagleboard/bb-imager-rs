use bb_imager_ui::{Message, customization};
use iced::widget::combo_box;

struct State(customization::State);

impl State {
    fn new() -> (Self, iced::Task<Message<()>>) {
        let res = Self(customization::State {
            customization: customization::Customization::CloudInit(Default::default()),
            default_username: "myuser",
            default_timezone: Some(chrono_tz::Tz::Asia__Kolkata),
            default_keymap: "us",
            timezones: combo_box::State::new(chrono_tz::TZ_VARIANTS.to_vec()),
            keymaps: combo_box::State::new(vec!["ua", "us", "uz", "vn", "za"]),
        });

        (res, iced::Task::none())
    }
}

fn main() {
    let app = iced::application(
        State::new,
        |s: &mut State, msg| {
            if let Message::UpdateCustomizaton(c) = msg {
                s.0.customization = c
            }
            iced::Task::none()
        },
        view,
    );

    bb_imager_ui::application(app).run().unwrap()
}

fn view(s: &State) -> iced::Element<'_, Message<()>> {
    customization::view(&s.0, iced::widget::Id::unique())
}
