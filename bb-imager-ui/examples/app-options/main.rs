use bb_imager_ui::{Message, app_options};

struct State(app_options::State);

impl State {
    fn new() -> (Self, iced::Task<Message<()>>) {
        let res = State(app_options::State {
            previous_page: app_options::PreviousPage::Flashing {
                has_customization: true,
            },
            cache_dir:
                "/var/home/ayush/.var/app/org.beagleboard.imagingutility/cache/imagingutility"
                    .into(),
            log_file: "/var/home/ayush/.var/app/org.beagleboard.imagingutility/cache/org.beagleboard.imagingutility.log".into(),
            license: iced::widget::text_editor::Content::with_text(include_str!("../../../LICENSE"))
        });

        (res, iced::Task::none())
    }
}

fn main() {
    let app = iced::application(State::new, |_: &mut State, _| iced::Task::none(), view);
    bb_imager_ui::application(app).run().unwrap()
}

fn view(s: &State) -> iced::Element<'_, Message<()>> {
    app_options::view(&s.0, iced::widget::Id::unique())
}
