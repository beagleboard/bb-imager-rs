use bb_imager_ui::{Message, destination_selection};

/// Stands in for the hardware enumeration the real app re-runs every second.
const DEVICES: [(&str, &str, u64); 4] = [
    ("/dev/sdb", "Generic MassStorageClass", 31_914_983_424),
    ("/dev/sdc", "SanDisk Ultra", 63_864_569_856),
    ("/dev/sdd", "Kingston DataTraveler", 15_931_539_456),
    (
        "/dev/nvme0n1",
        "Internal NVMe (hidden unless unfiltered)",
        1_024_209_543_168,
    ),
];

fn pretty_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];

    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }

    format!("{size:.2} {}", UNITS[unit])
}

struct State {
    inner: destination_selection::State,
}

impl State {
    fn new() -> (Self, iced::Task<Message>) {
        let mut inner = destination_selection::State {
            // `Some`, so the page renders its own "Save To File" row.
            image_file_name: Some("beagley-ai-debian-13.img".into()),
            instruction: Some(
                "1. Connect the board to your computer via USB.\n\
                 2. Hold the USER button while powering it on.\n\
                 3. The board should then appear in this list."
                    .into(),
            ),
            ..Default::default()
        };
        inner.destinations = devices(inner.filter_destination, &inner.search);

        (Self { inner }, iced::Task::none())
    }

    /// The host re-queries on every filter/search change; do the same here so
    /// both controls visibly do something.
    fn refresh(&mut self) {
        self.inner.destinations = devices(self.inner.filter_destination, &self.inner.search);
    }
}

fn devices(filter: bool, search: &str) -> Box<[destination_selection::DestinationItem]> {
    let search = search.to_lowercase();

    DEVICES
        .iter()
        .enumerate()
        // Pretend the last entry is an internal disk that the filter hides.
        .filter(|(i, _)| !filter || *i < DEVICES.len() - 1)
        .map(
            |(_, (ident, label, size))| destination_selection::DestinationItem {
                id: (*ident).into(),
                label: (*label).into(),
                subtitle: Some(pretty_bytes(*size).into()),
            },
        )
        .filter(|x| search.is_empty() || x.label.to_lowercase().contains(&search))
        .collect()
}

fn select(ident: &str) -> Option<destination_selection::DestinationDetails> {
    let (_, label, size) = DEVICES.iter().find(|(i, _, _)| *i == ident)?;

    Some(destination_selection::DestinationDetails {
        id: destination_selection::DestId::Device(ident.into()),
        title: (*label).into(),
        details: vec![
            ("Path".into(), ident.into()),
            ("Size".into(), pretty_bytes(*size).into()),
        ]
        .into(),
    })
}

fn main() {
    let app = iced::application(
        State::new,
        |s: &mut State, msg| {
            match msg {
                Message::UpdateSearchText(search) => {
                    s.inner.search = search;
                    s.refresh();
                }
                Message::DestinationFilter(x) => {
                    s.inner.filter_destination = x;
                    s.refresh();
                }
                Message::SelectDest(ident) => s.inner.selected = select(&ident),
                Message::SelectFileDest(name) => {
                    // The host opens a save dialog here; pretend it was accepted.
                    s.inner.selected = Some(destination_selection::DestinationDetails {
                        id: destination_selection::DestId::SaveToFile,
                        title: "Save To File".into(),
                        details: vec![("Path".into(), format!("/tmp/{name}").into())].into(),
                    });
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
    destination_selection::view(&s.inner, iced::widget::Id::unique())
}
