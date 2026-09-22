use bb_imager_ui::{Message, board_selection, flash_cancel};

struct State {
    /// Empty: the literal board below has no remote icon, so the pane falls back
    /// to the bundled one. `preview-board-selection` covers the fetched path.
    cache: bb_iced_widgets::cached_icon::Cache<std::sync::Arc<url::Url>>,
    inner: flash_cancel::State,
}

impl State {
    fn new() -> (Self, iced::Task<Message>) {
        let res = Self {
            cache: Default::default(),
            inner: flash_cancel::State {
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
}

fn main() {
    let app = iced::application(State::new, |_: &mut State, _| iced::Task::none(), view);
    bb_imager_ui::application(app).run().unwrap()
}

fn view(s: &State) -> iced::Element<'_, Message> {
    flash_cancel::view(&s.cache, &s.inner, iced::widget::Id::unique())
}
