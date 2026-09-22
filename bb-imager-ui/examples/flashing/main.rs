use std::time::{Duration, Instant};

use bb_imager_ui::{board_selection, flashing};

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
    /// Empty: the literal board below has no remote icon, so the pane falls back
    /// to the bundled one. `preview-board-selection` covers the fetched path.
    cache: bb_iced_widgets::cached_icon::Cache<std::sync::Arc<url::Url>>,
    /// Drives the phase cycle; runs 0.0 -> 4.0 and wraps.
    step: f32,
    inner: flashing::State,
}

impl State {
    fn new() -> (Self, iced::Task<Message>) {
        let res = Self {
            cache: Default::default(),
            step: 0.0,
            inner: flashing::State {
                progress: Default::default(),
                start_timestamp: None,
                board: board_selection::BoardDetails {
                    id: 0,
                    name: "BeagleY-AI".into(),
                    icon: None,
                    description: "Affordable single board computer with a quad-core 64-bit Arm \
                                  CPU and an integrated AI accelerator."
                        .into(),
                    specification: vec![
                        ("SoC".into(), "TI AM67A".into()),
                        ("CPU".into(), "Quad-core Arm Cortex-A53".into()),
                        ("RAM".into(), "4 GB LPDDR4".into()),
                        ("Connectivity".into(), "Wi-Fi 6 / Bluetooth 5.4".into()),
                    ]
                    .into(),
                    buttons: [
                        (
                            "Documentation",
                            url::Url::parse("https://docs.beagleboard.org/boards/beagley/ai/")
                                .unwrap(),
                        ),
                        (
                            "OSHW",
                            url::Url::parse("https://certification.oshwa.org/us002787.html")
                                .unwrap(),
                        ),
                    ]
                    .into(),
                },
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
        self.step = (self.step + 0.02) % 4.0;

        self.inner.progress = match self.step {
            s if s < 1.0 => flashing::Progress::Preparing,
            s if s < 2.0 => flashing::Progress::Downloading(s - 1.0),
            s if s < 3.0 => flashing::Progress::Flashing(s - 2.0),
            s if s < 3.5 => flashing::Progress::Verifying,
            _ => flashing::Progress::Customizing,
        };

        // The host stamps this on the first byte-moving update; mimic that so
        // the ETA has an elapsed time to extrapolate from.
        match self.inner.progress {
            flashing::Progress::Downloading(_) | flashing::Progress::Flashing(_) => {
                self.inner.start_timestamp.get_or_insert_with(Instant::now);
            }
            flashing::Progress::Preparing => self.inner.start_timestamp = None,
            _ => {}
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
    flashing::view(&s.cache, &s.inner, iced::widget::Id::unique()).map(Into::into)
}
