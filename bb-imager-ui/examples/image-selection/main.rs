use bb_config::config;
use bb_imager_ui::{Message, image_selection};

struct State {
    cache: bb_iced_widgets::cached_icon::Cache<std::sync::Arc<url::Url>>,
    inner: image_selection::State,
    catalog: Catalog,
    _tmpdir: tempfile::TempDir,
}

/// Every item tagged with the sublist it lives in; `None` is the root.
///
/// Stands in for the sqlite queries the real app uses to resolve a level.
struct Catalog(Vec<(Option<i64>, image_selection::ImageItem)>);

impl Catalog {
    fn items(&self, parent: Option<i64>) -> Vec<image_selection::ImageItem> {
        self.0
            .iter()
            .filter(|(p, _)| *p == parent)
            .map(|(_, x)| x.clone())
            .collect()
    }

    /// The sublist holding `sublist`, i.e. where "Back" leads.
    fn parent(&self, sublist: i64) -> Option<i64> {
        self.0
            .iter()
            .find(|(_, x)| matches!(x.id, image_selection::ImageId::OsSublist((id, _)) if id == sublist))
            .and_then(|(p, _)| *p)
    }
}

fn item(
    id: image_selection::ImageId,
    label: &'static str,
    icon: Option<std::sync::Arc<url::Url>>,
) -> image_selection::ImageItem {
    image_selection::ImageItem {
        id,
        icon,
        label: label.into(),
    }
}

impl State {
    fn new() -> (Self, iced::Task<Message>) {
        let cache_dir = tempfile::tempdir().unwrap();
        let downloader = bb_downloader::Downloader::new(cache_dir.as_ref()).unwrap();

        let logo = std::sync::Arc::new(
            url::Url::parse("https://www.debian.org/logos/openlogo-nd.svg").unwrap(),
        );

        use image_selection::ImageId;
        let catalog = Catalog(vec![
            (
                None,
                item(ImageId::OsImage(1), "Debian 13 XFCE", Some(logo.clone())),
            ),
            (
                None,
                item(
                    ImageId::OsSublist((10, config::Flasher::SdCard)),
                    "Older Debian Images",
                    Some(logo.clone()),
                ),
            ),
            (
                Some(10),
                item(ImageId::OsImage(2), "Debian 13 Minimal", Some(logo.clone())),
            ),
            (
                Some(10),
                item(
                    ImageId::OsSublist((11, config::Flasher::SdCard)),
                    "Debian 12 (Bookworm)",
                    Some(logo.clone()),
                ),
            ),
            (
                Some(11),
                item(ImageId::OsImage(3), "Debian 12 XFCE", Some(logo.clone())),
            ),
        ]);

        // One task per distinct icon, as the host does.
        let tasks = [logo].map(|u| {
            let downloader = downloader.clone();
            let url = url::Url::clone(&u);
            iced::Task::perform(
                async move { downloader.download(url).await.unwrap() },
                |p| Message::ResolveImage(u, p),
            )
        });

        let mut res = Self {
            cache: Default::default(),
            _tmpdir: cache_dir,
            inner: Default::default(),
            catalog,
        };
        res.goto(None);

        (res, iced::Task::batch(tasks))
    }

    /// Mirrors the host's `update_images`: the synthetic rows are appended at
    /// every level, not stored in the catalog.
    fn goto(&mut self, pos: Option<i64>) {
        let mut imgs = self.catalog.items(pos);
        imgs.push(item(
            image_selection::ImageId::Format,
            "Format SD Card",
            None,
        ));
        imgs.push(item(
            image_selection::ImageId::Local(config::Flasher::SdCard),
            "Select Local Image",
            None,
        ));

        self.inner.images = imgs.into();
        self.inner.pos = pos;
    }

    fn select(&self, id: image_selection::ImageId) -> Option<image_selection::ImageDetails> {
        let label = self
            .inner
            .images
            .iter()
            .find(|x| x.id == id)
            .map(|x| x.label.clone())?;

        let remote = self
            .inner
            .images
            .iter()
            .find(|x| x.id == id)
            .and_then(|x| x.icon.clone());

        Some(image_selection::ImageDetails {
            id,
            icon: match (id, remote) {
                (image_selection::ImageId::Format, _) => image_selection::ImageIcon::Format,
                (image_selection::ImageId::Local(_), _) => image_selection::ImageIcon::Local,
                (_, Some(u)) => image_selection::ImageIcon::Remote(u),
                (_, None) => image_selection::ImageIcon::Local,
            },
            title: label.as_ref().into(),
            description: Some(
                "Debian 13 (Trixie) with the Xfce Desktop for BeagleY-AI, based on the TI AM67A."
                    .into(),
            ),
            details: [
                ("Release Date", "2026-07-12".into()),
                ("Image Size", "12.00 GiB".into()),
                ("Download Size", "3.20 GiB".into()),
            ]
            .into(),
            // Two formats so the picker renders rather than a plain line.
            init_formats: &[config::InitFormat::Sysconf, config::InitFormat::CloudInit],
            init_format: config::InitFormat::Sysconf,
            buttons: [(
                "Support",
                url::Url::parse("https://forum.beagleboard.org/").unwrap(),
            )]
            .into(),
        })
    }
}

fn main() {
    let app = iced::application(
        State::new,
        |s: &mut State, msg| {
            match msg {
                Message::UpdateSearchText(search) => s.inner.search = search,
                Message::ResolveImage(u, p) => s.cache.insert(u, p),
                Message::SelectOs(image_selection::ImageId::OsSublist((id, _))) => s.goto(Some(id)),
                Message::SelectOs(id) => s.inner.selected = s.select(id),
                Message::GotoOsListParent => {
                    // "Back" only renders inside a sublist.
                    let parent = s.catalog.parent(s.inner.pos.unwrap());
                    s.goto(parent);
                }
                Message::UpdateInitFormat(f) => {
                    if let Some(x) = s.inner.selected.as_mut() {
                        x.init_format = f;
                    }
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
    image_selection::view(&s.cache, &s.inner, iced::widget::Id::unique())
}
