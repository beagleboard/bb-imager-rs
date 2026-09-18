use bb_imager_ui::{Message, configuration};
use iced::widget::combo_box;

struct State(configuration::State);

impl State {
    fn new() -> (Self, iced::Task<Message>) {
        let res = State(configuration::State {
            customization: configuration::Customization::CloudInit(Default::default()),
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
            match msg {
                Message::UpdateCustomization(c) => s.0.customization = c,
                Message::Reset => {
                    s.0.customization = configuration::Customization::CloudInit(Default::default())
                }
                _ => {}
            }
            iced::Task::none()
        },
        view,
    );

    bb_imager_ui::application(app).run().unwrap()
}

fn view(s: &State) -> iced::Element<'_, Message> {
    configuration::view(&s.0, iced::widget::Id::unique())
}
