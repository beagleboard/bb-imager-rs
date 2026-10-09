use bb_imager_ui::{Message, format_media_fail};
use iced::widget::text_editor::{Action, Motion};

struct State(format_media_fail::State);

impl State {
    fn new() -> (Self, iced::Task<Message>) {
        let mut logs = iced::widget::text_editor::Content::with_text(LOGS.trim());
        logs.perform(Action::Move(Motion::DocumentEnd));
        let res = State(format_media_fail::State {
            reason: "Fail Reason for Testing".into(),
            logs,
        });

        (res, iced::Task::none())
    }
}

fn main() {
    let app = iced::application(
        State::new,
        |s: &mut State, msg| {
            if let Message::EditorEvent(evt) = msg {
                match evt {
                    iced::widget::text_editor::Action::Edit(_) => {}
                    _ => s.0.logs.perform(evt),
                }
            }
            iced::Task::none()
        },
        view,
    );
    bb_imager_ui::application(app).run().unwrap()
}

fn view(s: &State) -> iced::Element<'_, Message> {
    format_media_fail::view(&s.0)
}

const LOGS: &str = r#"
2026-10-09T10:12:03.114233Z  INFO bb_imager_gui: Starting Format Process
2026-10-09T10:12:03.114301Z  INFO bb_imager_gui: Selected Destination: SdCard(Target { path: "/dev/sdb" })
2026-10-09T10:12:04.902117Z ERROR bb_imager_gui: Formatting failed with error: Failed to open destination
"#;
