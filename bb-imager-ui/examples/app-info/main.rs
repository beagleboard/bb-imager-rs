use bb_imager_ui::{Message, app_info};

struct State(app_info::State);

impl State {
    fn new() -> (Self, iced::Task<Message>) {
        let res = State(app_info::State { 
            app_name: "BeagleBoard Imager", 
            app_release: env!("CARGO_PKG_VERSION"), 
            app_desc: env!("CARGO_PKG_DESCRIPTION"), 
            license: iced::widget::text_editor::Content::with_text(include_str!("../../../LICENSE")),
            cache_dir:
                "/var/home/ayush/.var/app/org.beagleboard.imagingutility/cache/imagingutility"
                    .into(),
            log_path: "/var/home/ayush/.var/app/org.beagleboard.imagingutility/cache/org.beagleboard.imagingutility.log".into(),
        });

        (res, iced::Task::none())
    }
}

fn main() {
    let app = iced::application(State::new, |_: &mut State, _| iced::Task::none(), view);
    bb_imager_ui::application(app).run().unwrap()
}

fn view(s: &State) -> iced::Element<'_, Message> {
    app_info::view(&s.0, iced::widget::Id::unique())
}
