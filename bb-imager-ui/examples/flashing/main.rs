use std::time::{Duration, Instant};

use bb_imager_ui::flashing;

/// The page's own messages are folded into [`Message::Ui`]; the ticks driving
/// the fake progress are this example's own.
#[derive(Debug, Clone)]
enum Message {
    Ui,
    Tick,
}

impl From<bb_imager_ui::Message> for Message {
    fn from(_: bb_imager_ui::Message) -> Self {
        Self::Ui
    }
}

struct State {
    /// Drives each destination's phase cycle; each runs 0.0 -> 4.0 and wraps.
    steps: [f32; 2],
    inner: flashing::State,
}

impl State {
    fn new() -> (Self, iced::Task<Message>) {
        let res = Self {
            // Offset so the two destinations sit in different phases.
            steps: [0.0, 0.5],
            inner: flashing::State {
                destinations: [
                    ("Test SD Card (32 GB)".into(), Default::default()),
                    ("Test USB Drive (64 GB)".into(), Default::default()),
                ]
                .into(),
                start_timestamp: None,
            },
        };

        (res, iced::Task::none())
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(Duration::from_millis(100)).map(|_| Message::Tick)
    }

    /// Walk every phase so the preview exercises all five labels, the filling
    /// circle and the ETA line.
    fn tick(&mut self) {
        for (step, (_, progress)) in self.steps.iter_mut().zip(&mut self.inner.destinations) {
            *step = (*step + 0.02) % 4.0;

            *progress = match *step {
                s if s < 1.0 => flashing::Progress::Preparing,
                s if s < 2.0 => flashing::Progress::Writing(s - 1.0),
                s if s < 3.0 => flashing::Progress::Verifying,
                _ => flashing::Progress::Customizing,
            };
        }

        // The host stamps this on the first byte-moving update; mimic that so
        // the ETA has an elapsed time to extrapolate from.
        let progresses = || self.inner.destinations.iter().map(|(_, p)| p);
        if progresses().any(|p| matches!(p, flashing::Progress::Writing(_))) {
            self.inner.start_timestamp.get_or_insert_with(Instant::now);
        } else if progresses().all(|p| matches!(p, flashing::Progress::Preparing)) {
            self.inner.start_timestamp = None;
        }
    }
}

fn main() {
    let app = iced::application(
        State::new,
        |s: &mut State, msg| {
            if let Message::Tick = msg {
                s.tick();
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
    flashing::view(&s.inner, iced::widget::Id::unique()).map(Into::into)
}
