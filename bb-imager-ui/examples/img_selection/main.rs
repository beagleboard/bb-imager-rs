use bb_config::config;
use bb_imager_ui::{Message, img_selection};

struct State {
    cache: bb_iced_widgets::cached_icon::Cache<std::sync::Arc<url::Url>>,
    inner: img_selection::State,
    catalog: Catalog,
    _tmpdir: tempfile::TempDir,
}

/// Flat list of every item, each tagged with the sublist it lives in. [`None`] is the root list.
///
/// Stands in for the sqlite queries the real app uses to resolve a sublist.
struct Catalog(Vec<(Option<i64>, img_selection::ImageItem)>);

impl Catalog {
    fn items(&self, parent: Option<i64>) -> Box<[img_selection::ImageItem]> {
        self.0
            .iter()
            .filter(|(p, _)| *p == parent)
            .map(|(_, x)| x.clone())
            .collect()
    }

    /// Sublist holding `sublist`, i.e. where "Back" leads to.
    fn parent(&self, sublist: i64) -> Option<i64> {
        self.0
            .iter()
            .find(|(_, x)| {
                matches!(x.id, img_selection::ImageId::OsSublist((id, _)) if id == sublist)
            })
            .and_then(|(p, _)| *p)
    }
}

impl State {
    fn new() -> (Self, iced::Task<Message<()>>) {
        let cache_dir = tempfile::tempdir().unwrap();
        let downloader = bb_downloader::Downloader::new(cache_dir.as_ref()).unwrap();

        let debian_logo = std::sync::Arc::new(
            url::Url::parse("https://www.debian.org/logos/openlogo-nd.svg").unwrap(),
        );

        let catalog = Catalog(vec![
            (
                None,
                img_selection::ImageItem {
                    id: img_selection::ImageId::OsImage(1),
                    label: "BeagleY-AI Debian 13 v7.1.x-k3 XFCE".into(),
                    icon: Some(debian_logo.clone()),
                    description: "Debian 13 (Trixie) with the Xfce Desktop for BeagleY-AI based on TI AM67A (J722S) processor running linux 7.1.".into(),
                    size: Some(12884901888),
                    release_date: Some(chrono::NaiveDate::from_ymd_opt(2026, 7, 12).unwrap()),
                },
            ),
            (
                None,
                img_selection::ImageItem {
                    id: img_selection::ImageId::OsSublist((10, config::Flasher::SdCard)),
                    label: "Older Debian Images".into(),
                    icon: Some(debian_logo.clone()),
                    description: "Previous Debian releases for BeagleY-AI.".into(),
                    size: None,
                    release_date: None,
                },
            ),
            (
                None,
                img_selection::ImageItem {
                    id: img_selection::ImageId::Local(config::Flasher::SdCard),
                    label: "Select Local Image".into(),
                    icon: None,
                    description: "".into(),
                    size: None,
                    release_date: None,
                },
            ),
            (
                None,
                img_selection::ImageItem {
                    id: img_selection::ImageId::Format,
                    label: "Format SD Card".into(),
                    icon: None,
                    description: "".into(),
                    size: None,
                    release_date: None,
                },
            ),
            (
                Some(10),
                img_selection::ImageItem {
                    id: img_selection::ImageId::OsImage(2),
                    label: "BeagleY-AI Debian 13 v7.1.x-k3 Minimal".into(),
                    icon: Some(debian_logo.clone()),
                    description: "Debian 13 (Trixie) console only image for BeagleY-AI based on TI AM67A (J722S) processor running linux 7.1.".into(),
                    size: Some(4294967296),
                    release_date: Some(chrono::NaiveDate::from_ymd_opt(2026, 7, 12).unwrap()),
                },
            ),
            (
                Some(10),
                img_selection::ImageItem {
                    id: img_selection::ImageId::OsSublist((11, config::Flasher::SdCard)),
                    label: "Debian 12 (Bookworm)".into(),
                    icon: Some(debian_logo.clone()),
                    description: "Nested sublist. Use it to check that going back walks up one level at a time.".into(),
                    size: None,
                    release_date: None,
                },
            ),
            (
                Some(11),
                img_selection::ImageItem {
                    id: img_selection::ImageId::OsImage(3),
                    label: "BeagleY-AI Debian 12 v6.1.x-ti XFCE".into(),
                    icon: Some(debian_logo.clone()),
                    description: "Debian 12 (Bookworm) with the Xfce Desktop for BeagleY-AI based on TI AM67A (J722S) processor running linux 6.1.".into(),
                    size: Some(11274289152),
                    release_date: Some(chrono::NaiveDate::from_ymd_opt(2025, 3, 18).unwrap()),
                },
            ),
            (
                Some(11),
                img_selection::ImageItem {
                    id: img_selection::ImageId::OsImage(4),
                    label: "BeagleY-AI Debian 12 v6.1.x-ti Minimal".into(),
                    icon: Some(debian_logo),
                    description: "Debian 12 (Bookworm) console only image for BeagleY-AI based on TI AM67A (J722S) processor running linux 6.1.".into(),
                    size: Some(3221225472),
                    release_date: Some(chrono::NaiveDate::from_ymd_opt(2025, 3, 18).unwrap()),
                },
            ),
        ]);

        let icons: std::collections::HashSet<_> = catalog
            .0
            .iter()
            .filter_map(|(_, x)| x.icon.clone())
            .collect();

        let tasks: Vec<_> = icons
            .into_iter()
            .map(|u| {
                let downloader = downloader.clone();
                let url = url::Url::clone(&u);
                iced::Task::perform(
                    async move { downloader.download(url).await.unwrap() },
                    |p| Message::ResolveImage(u, p),
                )
            })
            .collect();

        let res = Self {
            cache: bb_iced_widgets::cached_icon::Cache::default(),
            _tmpdir: cache_dir,
            inner: img_selection::State {
                imgs: catalog.items(None),
                search: "".into(),
                pos: None,
            },
            catalog,
        };

        (res, iced::Task::batch(tasks))
    }

    fn goto(&mut self, pos: Option<i64>) {
        self.inner.imgs = self.catalog.items(pos);
        self.inner.pos = pos;
    }
}

fn main() {
    let app = iced::application(
        State::new,
        |s: &mut State, msg| {
            match msg {
                Message::UpdateSearchText(search) => {
                    s.inner.search = search;
                }
                Message::ResolveImage(u, p) => {
                    s.cache.insert(u, p);
                }
                Message::SelectOs(img_selection::ImageId::OsSublist((id, _))) => {
                    s.goto(Some(id));
                }
                Message::GotoOsListParent => {
                    // "Back" is only rendered inside a sublist.
                    let parent = s.catalog.parent(s.inner.pos.unwrap());
                    s.goto(parent);
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
    img_selection::view(&s.cache, &s.inner, iced::widget::Id::unique())
}
