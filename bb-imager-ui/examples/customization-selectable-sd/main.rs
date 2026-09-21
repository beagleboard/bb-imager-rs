use bb_imager_ui::{Message, customization};
use iced::widget::combo_box;

struct State(customization::State);

impl State {
    fn new() -> (Self, iced::Task<Message<()>>) {
        let res = Self(customization::State {
            customization: customization::Customization::SelectableSd(Default::default()),
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
                Message::UpdateCustomizaton(c) => s.0.customization = c,
                Message::SelectInitFormat(x) => match x {
                    bb_config::config::InitFormat::None => {
                        s.0.customization = customization::Customization::SelectableSd(
                            customization::SelectableSd::None,
                        )
                    }
                    bb_config::config::InitFormat::Sysconf => {
                        s.0.customization = customization::Customization::SelectableSd(
                            customization::SelectableSd::SysConfig(Default::default()),
                        )
                    }
                    bb_config::config::InitFormat::CloudInit => {
                        s.0.customization = customization::Customization::SelectableSd(
                            customization::SelectableSd::CloudInit(Default::default()),
                        )
                    }
                    _ => unimplemented!(),
                },
                _ => {}
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
