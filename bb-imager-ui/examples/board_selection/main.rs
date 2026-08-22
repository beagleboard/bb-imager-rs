use bb_imager_ui::{Message, board_selection};

struct State {
    /// Filled asynchronously as the icon downloads land, exactly as the host does.
    cache: bb_iced_widgets::cached_icon::Cache<std::sync::Arc<url::Url>>,
    inner: board_selection::State,
    _tmpdir: tempfile::TempDir,
}

impl State {
    fn new() -> (Self, iced::Task<Message<()>>) {
        let cfg = serde_json::from_slice::<bb_config::config::Config>(include_bytes!(
            "../../../config.json"
        ))
        .expect("Failed to parse config");

        let cache_dir = tempfile::tempdir().unwrap();
        let downloader = bb_downloader::Downloader::new(cache_dir.as_ref()).unwrap();

        let iter: Vec<_> = cfg
            .imager
            .devices
            .iter()
            .flat_map(|x| x.icon.clone().map(std::sync::Arc::new))
            .collect();
        let tasks = iter.into_iter().map(|u| {
            let downloader = downloader.clone();
            // The downloader takes an owned `Url`, so this one clone stays.
            let url = url::Url::clone(&u);
            iced::Task::perform(
                async move { downloader.download(url).await.unwrap() },
                |p| Message::ResolveImage(u, p),
            )
        });

        let boards = cfg
            .imager
            .devices
            .into_iter()
            .enumerate()
            .map(|(id, x)| board_selection::Board {
                id: id as i64,
                icon: x.icon.map(std::sync::Arc::new),
                name: x.name,
            })
            .collect();

        let res = Self {
            cache: bb_iced_widgets::cached_icon::Cache::default(),
            _tmpdir: cache_dir,
            inner: board_selection::State {
                boards,
                search: "".into(),
            },
        };

        (res, iced::Task::batch(tasks))
    }
}

fn main() {
    let app = iced::application(
        State::new,
        |s: &mut State, msg| {
            match msg {
                Message::UpdateSearchText(search) => {
                    // The host re-runs the query; filtering in place is close enough
                    // to preview the box.
                    s.inner.search = search;
                }
                Message::ResolveImage(u, p) => {
                    s.cache.insert(u, p);
                }
                _ => {}
            };
            iced::Task::none()
        },
        view,
    );

    bb_imager_ui::application(app).run().unwrap()
}

fn view(s: &State) -> iced::Element<'_, Message<()>> {
    board_selection::view(&s.cache, &s.inner, iced::widget::Id::unique())
}
