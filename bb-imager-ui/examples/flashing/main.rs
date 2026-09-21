use std::time::Duration;

use bb_imager_ui::flashing;

#[derive(Debug, Clone)]
enum Message {
    Ui,
    UpdateProgress,
}

impl From<bb_imager_ui::Message<()>> for Message {
    fn from(_: bb_imager_ui::Message<()>) -> Self {
        Self::Ui
    }
}

struct State {
    prog: f32,
    inner: flashing::State,
}

impl State {
    fn new() -> (Self, iced::Task<Message>) {
        let res = Self {
            prog: 0.0,
            inner: flashing::State {
                has_customization: true,
                ..Default::default()
            },
        };

        (res, iced::Task::none())
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(Duration::from_millis(100)).map(|_| Message::UpdateProgress)
    }
}

fn main() {
    let app = iced::application(
        State::new,
        |s: &mut State, msg| {
            if let Message::UpdateProgress = msg {
                if s.prog < 1.0 {
                    s.prog += 0.04;
                    s.inner.progress = flashing::Progress::Preparing;
                } else if s.prog < 1.98 {
                    s.prog += 0.01;
                    s.inner.progress = flashing::Progress::Writing(s.prog - 1.0);
                } else if s.prog < 3.0 {
                    s.inner.progress = flashing::Progress::Customizing;
                    s.prog += 0.02;
                } else {
                    s.prog = 0.0;
                }
            }
            iced::Task::none()
        },
        view,
    );

    bb_imager_ui::application(app)
        .subscription(State::subscription)
        .run()
        .unwrap()
}

fn view(s: &State) -> iced::Element<'_, Message> {
    flashing::view(&s.inner).map(Into::into)
}
