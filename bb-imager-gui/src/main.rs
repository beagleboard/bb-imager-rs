#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

use bb_config::config;
use constants::PACKAGE_QUALIFIER;
use iced::{Subscription, Task, futures::SinkExt, widget};
use message::BBImagerMessage;
use tokio_stream::StreamExt as _;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod constants;
mod helpers;
mod message;
mod persistance;
mod ui;
mod updater;

fn main() -> iced::Result {
    let log_file_p = helpers::log_file_path();
    let log_file_dir = log_file_p.parent().unwrap();
    if !log_file_dir.is_dir() {
        std::fs::create_dir_all(log_file_dir).unwrap();
    }

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(std::fs::File::create(helpers::log_file_path()).unwrap()),
        )
        .try_init()
        .expect("Failed to register tracing_subscriber");

    tracing::info!("Resolved GUI keymap: {:?}", helpers::system_keymap());

    // Force using the low power gpu since this is not a GPU intensive application
    unsafe { std::env::set_var("WGPU_POWER_PREF", "low") };

    let icon =
        iced::window::icon::from_file_data(constants::WINDOW_ICON, Some(image::ImageFormat::Png))
            .ok();
    assert!(icon.is_some());

    #[cfg(target_os = "macos")]
    // HACK: mac_notification_sys set application name (not an option in notify-rust)
    let _ = notify_rust::set_application("org.beagleboard.imagingutility");

    let settings = iced::window::Settings {
        min_size: Some(constants::WINDOW_SIZE),
        size: constants::WINDOW_SIZE,
        ..Default::default()
    };

    iced::application(BBImager::new, message::update, ui::view)
        .title(app_title)
        .subscription(BBImager::subscription)
        .theme(BBImager::theme)
        .window(settings)
        .font(constants::FONT_EXTRA_LIGHT_BYTES)
        .font(constants::FONT_EXTRA_LIGHT_ITALIC_BYTES)
        .font(constants::FONT_LIGHT_BYTES)
        .font(constants::FONT_LIGHT_ITALIC_BYTES)
        .font(constants::FONT_NORMAL_BYTES)
        .font(constants::FONT_NORMAL_ITALIC_BYTES)
        .font(constants::FONT_MEDIUM_BYTES)
        .font(constants::FONT_MEDIUM_ITALIC_BYTES)
        .font(constants::FONT_SEMI_BOLD_BYTES)
        .font(constants::FONT_SEMI_BOLD_ITALIC_BYTES)
        .font(constants::FONT_BOLD_BYTES)
        .font(constants::FONT_BOLD_ITALIC_BYTES)
        .font(constants::FONT_EXTRA_BOLD_BYTES)
        .font(constants::FONT_EXTRA_BOLD_ITALIC_BYTES)
        .font(constants::FONT_BLACK_BYTES)
        .font(constants::FONT_BLACK_ITALIC_BYTES)
        .default_font(constants::FONT_REGULAR)
        .run()
}

fn app_title(_: &BBImager) -> String {
    if option_env!("PRE_RELEASE").is_some() {
        format!("{} (pre-release)", constants::APP_NAME)
    } else {
        format!("{} v{}", constants::APP_NAME, env!("CARGO_PKG_VERSION"))
    }
}

#[derive(Debug)]
struct BBImagerCommon {
    app_config: persistance::GuiConfiguration,
    boards: helpers::Boards,
    downloader: bb_downloader::Downloader,
    timezones: widget::combo_box::State<String>,
    keymaps: widget::combo_box::State<String>,

    // Constant image handles
    board_svg_handle: widget::svg::Handle,
    downloading_svg_handle: widget::svg::Handle,
    arrow_forward_svg_handle: widget::svg::Handle,
    format_svg_handle: widget::svg::Handle,
    file_add_svg_handle: widget::svg::Handle,
    arrow_back_svg_handle: widget::svg::Handle,
    usb_svg_handle: widget::svg::Handle,
    file_save_icon: widget::svg::Handle,
    info_svg_handle: widget::svg::Handle,
    window_icon_handle: widget::image::Handle,

    img_handle_cache: helpers::ImageHandleCache,
}

impl BBImagerCommon {
    fn updater_task(&self) -> Task<BBImagerMessage> {
        if cfg!(feature = "updater") {
            let downloader = self.downloader.clone();
            Task::perform(
                async move { updater::check_update(downloader).await },
                |x| match x {
                    Ok(Some(ver)) => BBImagerMessage::UpdateAvailable(ver),
                    Ok(None) => {
                        tracing::info!("Application is at the latest version");
                        BBImagerMessage::Null
                    }
                    Err(e) => {
                        tracing::error!("Failed to check for application update: {e:?}");
                        BBImagerMessage::Null
                    }
                },
            )
        } else {
            Task::none()
        }
    }

    fn fetch_images(&self, iter: impl IntoIterator<Item = url::Url>) -> Task<BBImagerMessage> {
        let tasks = iter.into_iter().map(|icon| {
            let downloader = self.downloader.clone();
            let icon_clone = icon.clone();
            let icon_clone2 = icon.clone();
            Task::perform(
                async move { downloader.download_no_cache(icon_clone, None).await },
                move |p| match p {
                    Ok(p) => BBImagerMessage::ResolveImage(icon_clone2, p),
                    Err(_) => {
                        tracing::warn!("Failed to fetch image {}", icon);
                        BBImagerMessage::Null
                    }
                },
            )
        });
        Task::batch(tasks)
    }

    fn fetch_board_images(&self) -> Task<BBImagerMessage> {
        // Do not try downloading same image multiple times
        let icons: HashSet<url::Url> = self
            .boards
            .devices()
            .filter_map(|(_, dev)| dev.icon.clone())
            .collect();

        self.fetch_images(icons)
    }

    fn fetch_os_images(&self, board: usize, target: &[usize]) -> Task<BBImagerMessage> {
        let Some(os_images) = self.boards.images(board, target) else {
            return Task::none();
        };

        // Do not try downloading same image multiple times
        let icons: HashSet<url::Url> = os_images.map(|(_, x)| x.icon()).cloned().collect();

        self.fetch_images(icons)
    }

    fn fetch_remote_subitems(&self, board: usize, target: &[usize]) -> Task<BBImagerMessage> {
        let Some(os_images) = self.boards.images(board, target) else {
            // Maybe resolving was missed
            if let config::OsListItem::RemoteSubList(item) = self.boards.image(target) {
                let url = item.subitems_url.clone();
                tracing::debug!("Downloading subitems from {:?}", url);

                let target_clone: Vec<usize> = target.to_vec();
                let downloader = self.downloader.clone();

                return Task::perform(
                    async move { downloader.download_json_no_cache(url).await },
                    move |x| match x {
                        Ok(item) => BBImagerMessage::ResolveRemoteSubitemItem {
                            item,
                            target: target_clone.clone(),
                        },
                        Err(e) => {
                            tracing::warn!("Failed to download subitems with error {e}");
                            BBImagerMessage::Null
                        }
                    },
                );
            } else {
                return Task::none();
            }
        };

        let remote_image_jobs = os_images
            .filter_map(|(idx, x)| {
                if let config::OsListItem::RemoteSubList(item) = x {
                    tracing::debug!("Fetch: {:?} at {}", item.subitems_url, idx);
                    Some((idx, item.subitems_url.clone()))
                } else {
                    None
                }
            })
            .map(|(idx, url)| {
                let mut new_target: Vec<usize> = target.to_vec();
                new_target.push(idx);

                let downloader = self.downloader.clone();
                let url_clone = url.clone();
                Task::perform(
                    async move {
                        downloader
                            .download_json_no_cache::<Vec<config::OsListItem>, url::Url>(url_clone)
                            .await
                    },
                    move |x| match x {
                        Ok(item) => BBImagerMessage::ResolveRemoteSubitemItem {
                            item,
                            target: new_target.clone(),
                        },
                        Err(e) => {
                            tracing::warn!("Failed to download subitems {:?} with error {e}", url);
                            BBImagerMessage::Null
                        }
                    },
                )
            });

        Task::batch(remote_image_jobs)
    }

    pub(crate) fn image_handle_cache(&self) -> &helpers::ImageHandleCache {
        &self.img_handle_cache
    }

    pub(crate) fn board_svg(&self) -> &widget::svg::Handle {
        &self.board_svg_handle
    }

    pub(crate) fn downloading_svg(&self) -> &widget::svg::Handle {
        &self.downloading_svg_handle
    }

    pub(crate) fn info_svg(&self) -> &widget::svg::Handle {
        &self.info_svg_handle
    }

    pub(crate) fn window_icon(&self) -> &widget::image::Handle {
        &self.window_icon_handle
    }
}

#[derive(Debug)]
struct ChooseBoardState {
    common: BBImagerCommon,
    selected_board: Option<usize>,
}

impl ChooseBoardState {
    pub(crate) fn devices(&self) -> impl Iterator<Item = (usize, &config::Device)> {
        self.common.boards.devices()
    }

    pub(crate) fn board_svg(&self) -> &widget::svg::Handle {
        &self.common.board_svg_handle
    }

    pub(crate) fn downloading_svg(&self) -> &widget::svg::Handle {
        &self.common.downloading_svg_handle
    }

    pub(crate) fn selected_board(&self) -> Option<&config::Device> {
        Some(self.common.boards.device(self.selected_board?))
    }

    pub(crate) fn image_handle_cache(&self) -> &helpers::ImageHandleCache {
        &self.common.img_handle_cache
    }
}

#[derive(Debug)]
struct ChooseOsState {
    common: BBImagerCommon,
    selected_board: usize,
    pos: Vec<usize>,
    selected_image: Option<(OsImageId, helpers::BoardImage)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OsImageId {
    // Vec points to parent
    Format(Vec<usize>),
    // Vec points to parent
    Local(Vec<usize>),
    // Vec points to OsImage
    Remote(Vec<usize>),
}

struct OsImageItem<'a> {
    id: OsImageId,
    icon: Option<&'a url::Url>,
    label: &'a str,
    is_sublist: bool,
}

impl<'a> OsImageItem<'a> {
    fn format(parent: Vec<usize>, label: &'a str) -> Self {
        Self {
            id: OsImageId::Format(parent),
            icon: None,
            label,
            is_sublist: false,
        }
    }

    fn local(parent: Vec<usize>) -> Self {
        Self {
            id: OsImageId::Local(parent),
            icon: None,
            label: "Select Local Image",
            is_sublist: false,
        }
    }

    fn remote(id: Vec<usize>, url: &'a url::Url, label: &'a str, is_sublist: bool) -> Self {
        Self {
            id: OsImageId::Remote(id),
            icon: Some(url),
            label,
            is_sublist,
        }
    }
}

impl ChooseOsState {
    pub(crate) fn selected_image(&self) -> Option<(&OsImageId, &helpers::BoardImage)> {
        match &self.selected_image {
            Some((x, y)) => Some((x, y)),
            None => None,
        }
    }

    pub(crate) fn selected_board(&self) -> &config::Device {
        self.common.boards.device(self.selected_board)
    }

    pub(crate) fn images(&self) -> Option<impl Iterator<Item = OsImageItem<'_>>> {
        let iter = self
            .common
            .boards
            .images(self.selected_board, self.pos.as_slice())?
            .map(|(id, x)| {
                let mut idx = self.pos.clone();
                idx.push(id);

                OsImageItem::remote(
                    idx,
                    x.icon(),
                    x.name(),
                    matches!(
                        x,
                        config::OsListItem::SubList(_) | config::OsListItem::RemoteSubList(_)
                    ),
                )
            });

        let extra = match self.flasher() {
            config::Flasher::SdCard => vec![
                OsImageItem::format(self.pos.clone(), "Format SD Card"),
                OsImageItem::local(self.pos.clone()),
            ],
            _ => vec![OsImageItem::local(self.pos.clone())],
        };

        Some(iter.chain(extra))
    }

    pub(crate) fn image(&self, idx: &[usize]) -> &config::OsListItem {
        self.common.boards.image(idx)
    }

    pub(crate) fn image_handle_cache(&self) -> &helpers::ImageHandleCache {
        &self.common.img_handle_cache
    }

    pub(crate) fn flasher(&self) -> config::Flasher {
        if self.pos.is_empty() {
            self.selected_board().flasher
        } else {
            match self.image(&self.pos) {
                config::OsListItem::Image(_) => panic!("Expected list"),
                config::OsListItem::SubList(x) => x.flasher,
                config::OsListItem::RemoteSubList(x) => x.flasher,
            }
        }
    }

    pub(crate) fn downloading_svg(&self) -> &widget::svg::Handle {
        &self.common.downloading_svg_handle
    }

    pub(crate) fn arrow_forward_svg(&self) -> &widget::svg::Handle {
        &self.common.arrow_forward_svg_handle
    }

    pub(crate) fn format_svg(&self) -> &widget::svg::Handle {
        &self.common.format_svg_handle
    }

    pub(crate) fn file_add_svg(&self) -> &widget::svg::Handle {
        &self.common.file_add_svg_handle
    }

    pub(crate) fn arrow_back_svg(&self) -> &widget::svg::Handle {
        &self.common.arrow_back_svg_handle
    }

    pub(crate) fn downloader(&self) -> &bb_downloader::Downloader {
        &self.common.downloader
    }
}

#[derive(Debug)]
enum DestinationItem<'a> {
    SaveToFile(String),
    Destination(&'a helpers::Destination),
}

impl<'a> std::fmt::Display for DestinationItem<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DestinationItem::SaveToFile(_) => write!(f, "Save To File"),
            DestinationItem::Destination(d) => d.fmt(f),
        }
    }
}

impl<'a> DestinationItem<'a> {
    fn msg(&'a self) -> BBImagerMessage {
        match self {
            DestinationItem::SaveToFile(x) => BBImagerMessage::SelectFileDest(x.clone()),
            DestinationItem::Destination(d) => BBImagerMessage::SelectDest((*d).clone()),
        }
    }

    fn is_selected(&'a self, dst: &'a helpers::Destination) -> bool {
        match self {
            DestinationItem::SaveToFile(_) => false,
            DestinationItem::Destination(d) => dst.eq(d),
        }
    }
}

#[derive(Debug)]
struct ChooseDestState {
    common: BBImagerCommon,
    selected_board: usize,
    selected_image: (OsImageId, helpers::BoardImage),
    selected_dest: Option<helpers::Destination>,
    destinations: Vec<helpers::Destination>,
}

impl ChooseDestState {
    fn selected_dest(&self) -> Option<&helpers::Destination> {
        self.selected_dest.as_ref()
    }

    fn destinations<'a>(&'a self) -> impl Iterator<Item = DestinationItem<'a>> + 'a {
        let iter = self.destinations.iter().map(DestinationItem::Destination);

        let temp = match self.selected_image.1.file_name() {
            Some(x) => vec![DestinationItem::SaveToFile(x)],
            None => vec![],
        };

        iter.chain(temp)
    }

    fn usb_svg(&self) -> &widget::svg::Handle {
        &self.common.usb_svg_handle
    }

    fn file_save_icon(&self) -> &widget::svg::Handle {
        &self.common.file_save_icon
    }

    pub(crate) fn selected_board(&self) -> &config::Device {
        self.common.boards.device(self.selected_board)
    }

    fn instruction(&self) -> Option<&str> {
        match self.selected_image.1.info_text() {
            Some(x) => Some(x),
            None => self.selected_board().instructions.as_deref(),
        }
    }
}

#[derive(Debug)]
struct CustomizeState {
    common: BBImagerCommon,
    selected_board: usize,
    selected_image: (OsImageId, helpers::BoardImage),
    selected_dest: helpers::Destination,
    customization: helpers::FlashingCustomization,
}

impl CustomizeState {
    fn timezones(&self) -> &widget::combo_box::State<String> {
        &self.common.timezones
    }

    fn keymaps(&self) -> &widget::combo_box::State<String> {
        &self.common.keymaps
    }

    fn customization(&self) -> &helpers::FlashingCustomization {
        &self.customization
    }

    fn app_config(&self) -> &persistance::GuiConfiguration {
        &self.common.app_config
    }

    fn save_app_config(&self) -> Task<BBImagerMessage> {
        let config = self.app_config().clone();
        Task::future(async move {
            if let Err(e) = config.save().await {
                tracing::error!("Failed to save config: {e}");
            }
            BBImagerMessage::Null
        })
    }

    fn selected_board(&self) -> &str {
        self.common.boards.device(self.selected_board).name.as_str()
    }

    fn selected_image(&self) -> String {
        self.selected_image.1.to_string()
    }

    fn selected_destination(&self) -> String {
        match self.selected_dest.size() {
            Some(x) => format!("{} ({})", self.selected_dest, helpers::pretty_bytes(x)),
            None => self.selected_dest.to_string(),
        }
    }

    fn modifications(&self) -> Vec<&'static str> {
        match self.customization() {
            helpers::FlashingCustomization::LinuxSdSysconfig(x) => {
                let mut ans = Vec::new();

                if x.user.is_some() {
                    ans.push("• User account configured");
                }

                if x.wifi.is_some() {
                    ans.push("• Wifi configured");
                }

                if x.hostname.is_some() {
                    ans.push("• Hostname configured");
                }

                if x.keymap.is_some() {
                    ans.push("• Keymap configured");
                }

                if x.timezone.is_some() {
                    ans.push("• Timezone configured");
                }

                if x.ssh.is_some() {
                    ans.push("• SSH Key configured");
                }

                if x.usb_enable_dhcp == Some(true) {
                    ans.push("• USB DHCP enabled");
                }

                ans
            }
            helpers::FlashingCustomization::Bcf(x) => {
                if !x.verify {
                    vec!["• Skip Verification"]
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        }
    }
}

#[derive(Debug)]
struct FlashingState {
    common: BBImagerCommon,
    selected_board: usize,
    selected_image: helpers::BoardImage,
    cancel_flashing: iced::task::Handle,
    progress: bb_flasher::DownloadFlashingStatus,
    start_timestamp: Option<Instant>,
}

impl FlashingState {
    pub(crate) fn selected_board(&self) -> &config::Device {
        self.common.boards.device(self.selected_board)
    }

    fn time_remaining(&self) -> Option<Duration> {
        const THRESHOLD: f32 = 0.02;

        match self.progress {
            bb_flasher::DownloadFlashingStatus::FlashingProgress(x)
            | bb_flasher::DownloadFlashingStatus::DownloadingProgress(x) => {
                if x < THRESHOLD {
                    None
                } else {
                    let t = self.start_timestamp?.elapsed();
                    let x = x.clamp(0.0, 1.0);
                    let scale = (1.0 - x) / x;
                    Some(t.mul_f32(scale))
                }
            }
            bb_flasher::DownloadFlashingStatus::Customizing => Some(Duration::from_secs(1)),
            _ => None,
        }
    }

    fn progress_update(&mut self, u: bb_flasher::DownloadFlashingStatus) {
        // Required for better time estimate.
        match u {
            bb_flasher::DownloadFlashingStatus::DownloadingProgress(_)
            | bb_flasher::DownloadFlashingStatus::FlashingProgress(_) => {
                if self.start_timestamp.is_none() {
                    self.start_timestamp = Some(Instant::now())
                }
            }
            _ => {}
        }

        self.progress = u;
    }
}

#[derive(Debug)]
struct FlashingFinishState {
    common: BBImagerCommon,
    selected_board: usize,
}

impl FlashingFinishState {
    pub(crate) fn selected_board(&self) -> &config::Device {
        self.common.boards.device(self.selected_board)
    }
}

struct FlashingFailState {
    common: BBImagerCommon,
    err: String,
    logs: widget::text_editor::Content,
}

// State for Pages that can be opened from any of the normal pages but are not part of normal flow.
// Eg: Application info
enum OverlayData {
    ChooseBoard(ChooseBoardState),
    ChooseOs(ChooseOsState),
    ChooseDest(ChooseDestState),
    Customize(CustomizeState),
    Review(CustomizeState),
    Flashing(FlashingState),
    FlashingCancel(FlashingFinishState),
    FlashingFail(FlashingFailState),
    FlashingSuccess(FlashingFinishState),
}

impl OverlayData {
    fn common_mut(&mut self) -> &mut BBImagerCommon {
        match self {
            Self::ChooseBoard(x) => &mut x.common,
            Self::ChooseOs(x) => &mut x.common,
            Self::ChooseDest(x) => &mut x.common,
            Self::Customize(x) => &mut x.common,
            Self::Review(x) => &mut x.common,
            Self::Flashing(x) => &mut x.common,
            Self::FlashingCancel(x) => &mut x.common,
            Self::FlashingFail(x) => &mut x.common,
            Self::FlashingSuccess(x) => &mut x.common,
        }
    }

    fn common(&self) -> &BBImagerCommon {
        match self {
            Self::ChooseBoard(x) => &x.common,
            Self::ChooseOs(x) => &x.common,
            Self::ChooseDest(x) => &x.common,
            Self::Customize(x) => &x.common,
            Self::Review(x) => &x.common,
            Self::Flashing(x) => &x.common,
            Self::FlashingCancel(x) => &x.common,
            Self::FlashingFail(x) => &x.common,
            Self::FlashingSuccess(x) => &x.common,
        }
    }
}

impl TryFrom<BBImager> for OverlayData {
    type Error = ();

    fn try_from(value: BBImager) -> Result<Self, Self::Error> {
        match value {
            BBImager::ChooseBoard(x) => Ok(Self::ChooseBoard(x)),
            BBImager::ChooseOs(x) => Ok(Self::ChooseOs(x)),
            BBImager::ChooseDest(x) => Ok(Self::ChooseDest(x)),
            BBImager::Customize(x) => Ok(Self::Customize(x)),
            BBImager::Review(x) => Ok(Self::Review(x)),
            BBImager::Flashing(x) => Ok(Self::Flashing(x)),
            BBImager::FlashingCancel(x) => Ok(Self::FlashingCancel(x)),
            BBImager::FlashingFail(x) => Ok(Self::FlashingFail(x)),
            BBImager::FlashingSuccess(x) => Ok(Self::FlashingSuccess(x)),
            BBImager::Dummy | BBImager::AppInfo(_) => Err(()),
        }
    }
}

impl From<OverlayData> for BBImager {
    fn from(value: OverlayData) -> Self {
        match value {
            OverlayData::ChooseBoard(x) => Self::ChooseBoard(x),
            OverlayData::ChooseOs(x) => Self::ChooseOs(x),
            OverlayData::ChooseDest(x) => Self::ChooseDest(x),
            OverlayData::Customize(x) => Self::Customize(x),
            OverlayData::Review(x) => Self::Review(x),
            OverlayData::Flashing(x) => Self::Flashing(x),
            OverlayData::FlashingCancel(x) => Self::FlashingCancel(x),
            OverlayData::FlashingFail(x) => Self::FlashingFail(x),
            OverlayData::FlashingSuccess(x) => Self::FlashingSuccess(x),
        }
    }
}

struct OverlayState {
    page: OverlayData,
    log_path: String,
    license: widget::text_editor::Content,
    cache_dir: String,
}

impl OverlayState {
    fn new(page: OverlayData) -> Self {
        let log_path = helpers::log_file_path().to_string_lossy().to_string();
        let license = widget::text_editor::Content::with_text(constants::APP_LINCESE);
        let cache_dir = helpers::project_dirs()
            .unwrap()
            .cache_dir()
            .to_string_lossy()
            .to_string();

        Self {
            page,
            log_path,
            license,
            cache_dir,
        }
    }

    fn common(&self) -> &BBImagerCommon {
        self.page.common()
    }

    fn common_mut(&mut self) -> &mut BBImagerCommon {
        self.page.common_mut()
    }

    fn log_path(&self) -> &str {
        &self.log_path
    }

    fn cache_dir_path(&self) -> &str {
        &self.cache_dir
    }
}

#[derive(Default)]
enum BBImager {
    // Dummy state to allow clone-free move among variants. Should never be exposed in view.
    #[default]
    Dummy,
    ChooseBoard(ChooseBoardState),
    ChooseOs(ChooseOsState),
    ChooseDest(ChooseDestState),
    Customize(CustomizeState),
    Review(CustomizeState),
    Flashing(FlashingState),
    FlashingCancel(FlashingFinishState),
    FlashingFail(FlashingFailState),
    FlashingSuccess(FlashingFinishState),
    AppInfo(OverlayState),
}

impl BBImager {
    const fn choose_board(common: BBImagerCommon) -> Self {
        Self::ChooseBoard(ChooseBoardState {
            common,
            selected_board: None,
        })
    }
}

impl BBImager {
    fn new() -> (Self, Task<BBImagerMessage>) {
        let app_config = persistance::GuiConfiguration::load().unwrap_or_default();

        let downloader = bb_downloader::Downloader::new(
            directories::ProjectDirs::from(
                PACKAGE_QUALIFIER.0,
                PACKAGE_QUALIFIER.1,
                PACKAGE_QUALIFIER.2,
            )
            .unwrap()
            .cache_dir()
            .to_path_buf(),
        )
        .unwrap();

        // Fetch old config
        let client = downloader.clone();
        let config_task = helpers::refresh_config_task(client, &helpers::Boards::new());
        let boards = helpers::Boards::new();

        let img_handle_cache = helpers::ImageHandleCache::from_iter(
            boards
                .devices()
                .filter_map(|(_, dev)| dev.icon.clone())
                .filter_map(|icon| {
                    let path = downloader.check_cache_from_url(icon.clone())?;
                    Some((icon, path))
                }),
        );

        let common = BBImagerCommon {
            app_config,
            downloader: downloader.clone(),
            timezones: widget::combo_box::State::new(
                constants::TIMEZONES.iter().map(|x| x.to_string()).collect(),
            ),
            keymaps: widget::combo_box::State::new(
                constants::KEYMAP_LAYOUTS
                    .iter()
                    .map(|x| x.to_string())
                    .collect(),
            ),
            boards,
            board_svg_handle: widget::svg::Handle::from_memory(constants::BOARD_ICON),
            downloading_svg_handle: widget::svg::Handle::from_memory(constants::DOWNLOADING_ICON),
            arrow_forward_svg_handle: widget::svg::Handle::from_memory(
                constants::ARROW_FORWARD_IOS_ICON,
            ),
            format_svg_handle: widget::svg::Handle::from_memory(constants::FORMAT_ICON),
            file_add_svg_handle: widget::svg::Handle::from_memory(constants::FILE_ADD_ICON),
            arrow_back_svg_handle: widget::svg::Handle::from_memory(constants::ARROW_BACK_ICON),
            usb_svg_handle: widget::svg::Handle::from_memory(constants::USB_ICON),
            file_save_icon: widget::svg::Handle::from_memory(constants::FILE_SAVE_ICON),
            info_svg_handle: widget::svg::Handle::from_memory(constants::INFO_ICON),
            window_icon_handle: widget::image::Handle::from_bytes(crate::constants::WINDOW_ICON),

            img_handle_cache,
        };

        // Fetch all board images
        let board_image_task = common.fetch_board_images();

        let updater_task = common.updater_task();
        (
            Self::choose_board(common),
            Task::batch([config_task, board_image_task, updater_task]),
        )
    }

    fn theme(&self) -> iced::Theme {
        iced::Theme::custom(
            "Beagle",
            iced::theme::Palette {
                background: constants::BACKGROUND,
                text: iced::Color::WHITE,
                primary: constants::TONGUE_ORANGE,
                success: constants::CHECK_MARK_GREEN,
                warning: constants::HAIR_LIGHT_BROWN,
                danger: constants::DANGER,
            },
        )
    }

    fn fetch_board_images(&self) -> Task<BBImagerMessage> {
        self.common().fetch_board_images()
    }

    fn boards_merge(&mut self, c: bb_config::Config) {
        self.common_mut().boards.merge(c)
    }

    fn common_mut(&mut self) -> &mut BBImagerCommon {
        match self {
            BBImager::ChooseBoard(x) => &mut x.common,
            BBImager::ChooseOs(x) => &mut x.common,
            BBImager::ChooseDest(x) => &mut x.common,
            BBImager::Customize(x) => &mut x.common,
            BBImager::Review(x) => &mut x.common,
            BBImager::Flashing(x) => &mut x.common,
            BBImager::FlashingCancel(x) => &mut x.common,
            BBImager::FlashingFail(x) => &mut x.common,
            BBImager::FlashingSuccess(x) => &mut x.common,
            BBImager::AppInfo(x) => x.common_mut(),
            BBImager::Dummy => panic!("Invalid State"),
        }
    }

    fn common(&self) -> &BBImagerCommon {
        match self {
            BBImager::ChooseBoard(x) => &x.common,
            BBImager::ChooseOs(x) => &x.common,
            BBImager::ChooseDest(x) => &x.common,
            BBImager::Customize(x) => &x.common,
            BBImager::Review(x) => &x.common,
            BBImager::Flashing(x) => &x.common,
            BBImager::FlashingCancel(x) => &x.common,
            BBImager::FlashingFail(x) => &x.common,
            BBImager::FlashingSuccess(x) => &x.common,
            BBImager::AppInfo(x) => x.common(),
            BBImager::Dummy => panic!("Invalid state"),
        }
    }

    fn image_cache_insert(&mut self, k: url::Url, v: std::path::PathBuf) {
        self.common_mut().img_handle_cache.insert(k, v)
    }

    fn resolve_remote_subitem(
        &mut self,
        item: Vec<bb_config::config::OsListItem>,
        target: &[usize],
    ) {
        self.common_mut()
            .boards
            .resolve_remote_subitem(item, target);
    }

    fn restart(&mut self) {
        *self = match std::mem::take(self) {
            BBImager::ChooseOs(x) => BBImager::choose_board(x.common),
            BBImager::ChooseDest(x) => BBImager::choose_board(x.common),
            BBImager::Customize(x) | BBImager::Review(x) => BBImager::choose_board(x.common),
            BBImager::Flashing(x) => BBImager::choose_board(x.common),
            BBImager::FlashingCancel(x) | BBImager::FlashingSuccess(x) => {
                BBImager::choose_board(x.common)
            }
            BBImager::FlashingFail(x) => BBImager::choose_board(x.common),
            BBImager::Dummy | BBImager::AppInfo(_) | BBImager::ChooseBoard(_) => {
                panic!("Unexpected screen")
            }
        };
    }

    fn subscription(&self) -> Subscription<BBImagerMessage> {
        match self {
            Self::ChooseDest(x) => {
                Subscription::run_with(x.selected_image.1.flasher(), |flasher| {
                    iced::futures::stream::unfold(*flasher, async move |f| {
                        let mut dest = helpers::destinations(f).await;

                        dest.sort_by_key(|x| x.to_string());

                        let msg = BBImagerMessage::Destinations(dest);
                        Some((msg, f))
                    })
                    .throttle(Duration::from_secs(1))
                })
            }
            _ => Subscription::none(),
        }
    }

    fn start_flashing(&mut self) -> Task<BBImagerMessage> {
        let state = match std::mem::take(self) {
            Self::Review(inner) => inner,
            _ => panic!("Unexpected page"),
        };

        let board = state.common.boards.device(state.selected_board);

        let customization = state.customization;
        let img = state.selected_image.1.clone();
        let dst = state.selected_dest;

        tracing::info!("Starting Flashing Process");
        tracing::info!("Selected Board: {:#?}", board);
        tracing::info!("Selected Image: {:#?}", img);
        tracing::info!("Selected Destination: {:#?}", dst);
        tracing::info!("Selected Customization: {:#?}", customization);

        let cancel = tokio_util::sync::CancellationToken::new();

        let s = iced::stream::channel(20, async move |mut chan| {
            let (tx, mut rx) = iced::futures::channel::mpsc::channel(19);

            let cancel_child = cancel.child_token();
            let flash_task = tokio::spawn(async move {
                helpers::flash(img, customization, dst, tx, cancel_child).await
            });
            let mut chan_clone = chan.clone();
            let progress_task = tokio::spawn(async move {
                while let Some(progress) = rx.next().await {
                    let _ = chan_clone.try_send(BBImagerMessage::FlashProgress(progress));
                }
            });
            let _guard = cancel.drop_guard();

            let res = flash_task
                .await
                .expect("Tokio runtime failed to spawn task");

            let res = match res {
                Ok(_) => {
                    tracing::info!("Flashing Successfull");
                    BBImagerMessage::FlashSuccess
                }
                Err(e) => {
                    tracing::error!("Flashing failed with error: {:#?}", e);
                    BBImagerMessage::FlashFail(e.to_string())
                }
            };

            let _ = chan.send(res).await;
            progress_task.abort();
        });

        let (t, h) = Task::stream(s).abortable();

        *self = Self::Flashing(FlashingState {
            common: state.common,
            selected_board: state.selected_board,
            selected_image: state.selected_image.1,
            cancel_flashing: h,
            progress: bb_flasher::DownloadFlashingStatus::Preparing,
            start_timestamp: None,
        });

        t
    }

    fn back(&mut self) {
        *self = match std::mem::take(self) {
            Self::ChooseOs(inner) => Self::ChooseBoard(ChooseBoardState {
                common: inner.common,
                selected_board: Some(inner.selected_board),
            }),
            Self::ChooseDest(inner) => Self::ChooseOs(ChooseOsState {
                common: inner.common,
                selected_board: inner.selected_board,
                pos: Vec::new(),
                selected_image: Some(inner.selected_image),
            }),
            Self::Customize(inner) => {
                if helpers::static_destination(inner.selected_image.1.flasher()).is_none() {
                    Self::ChooseDest(ChooseDestState {
                        common: inner.common,
                        selected_board: inner.selected_board,
                        selected_image: inner.selected_image,
                        selected_dest: Some(inner.selected_dest),
                        destinations: Vec::new(),
                    })
                } else {
                    Self::ChooseOs(ChooseOsState {
                        common: inner.common,
                        selected_board: inner.selected_board,
                        pos: Vec::new(),
                        selected_image: Some(inner.selected_image),
                    })
                }
            }
            Self::Review(inner) => {
                if helpers::no_customization(
                    inner.selected_image.1.flasher(),
                    &inner.selected_image.1,
                )
                .is_none()
                {
                    Self::Customize(inner)
                } else if helpers::static_destination(inner.selected_image.1.flasher()).is_none() {
                    Self::ChooseDest(ChooseDestState {
                        common: inner.common,
                        selected_board: inner.selected_board,
                        selected_image: inner.selected_image,
                        selected_dest: Some(inner.selected_dest),
                        destinations: Vec::new(),
                    })
                } else {
                    Self::ChooseOs(ChooseOsState {
                        common: inner.common,
                        selected_board: inner.selected_board,
                        pos: Vec::new(),
                        selected_image: Some(inner.selected_image),
                    })
                }
            }
            Self::AppInfo(inner) => inner.page.into(),
            Self::Dummy
            | Self::FlashingSuccess(_)
            | Self::FlashingFail(_)
            | Self::FlashingCancel(_)
            | Self::Flashing(_)
            | Self::ChooseBoard(_) => panic!("Unexpected message"),
        }
    }

    fn next(&mut self) -> Task<BBImagerMessage> {
        *self = match std::mem::take(self) {
            Self::ChooseBoard(inner) => {
                let selected_board = inner
                    .selected_board
                    .expect("Board should alread have been selected");
                Self::ChooseOs(ChooseOsState {
                    common: inner.common,
                    selected_board,
                    pos: Vec::with_capacity(5),
                    selected_image: None,
                })
            }
            Self::ChooseOs(inner) => {
                let selected_image = inner
                    .selected_image
                    .expect("Image should already be selected");

                if let Some(dest) = helpers::static_destination(selected_image.1.flasher()) {
                    if let Some(customization) =
                        helpers::no_customization(selected_image.1.flasher(), &selected_image.1)
                    {
                        Self::Customize(CustomizeState {
                            common: inner.common,
                            selected_board: inner.selected_board,
                            selected_image,
                            selected_dest: dest,
                            customization,
                        })
                    } else {
                        let temp = helpers::FlashingCustomization::new(
                            selected_image.1.flasher(),
                            &selected_image.1,
                            &inner.common.app_config,
                        );

                        Self::Customize(CustomizeState {
                            common: inner.common,
                            selected_board: inner.selected_board,
                            selected_image,
                            selected_dest: dest,
                            customization: temp,
                        })
                    }
                } else {
                    Self::ChooseDest(ChooseDestState {
                        common: inner.common,
                        selected_board: inner.selected_board,
                        selected_image,
                        selected_dest: None,
                        destinations: Vec::new(),
                    })
                }
            }
            Self::ChooseDest(inner) => {
                if let Some(customization) = helpers::no_customization(
                    inner.selected_image.1.flasher(),
                    &inner.selected_image.1,
                ) {
                    Self::Review(CustomizeState {
                        common: inner.common,
                        selected_board: inner.selected_board,
                        selected_image: inner.selected_image,
                        selected_dest: inner
                            .selected_dest
                            .expect("Destination should already be selected"),
                        customization,
                    })
                } else {
                    let temp = helpers::FlashingCustomization::new(
                        inner.selected_image.1.flasher(),
                        &inner.selected_image.1,
                        &inner.common.app_config,
                    );

                    Self::Customize(CustomizeState {
                        common: inner.common,
                        selected_board: inner.selected_board,
                        selected_image: inner.selected_image,
                        selected_dest: inner
                            .selected_dest
                            .expect("Destination should already be selected"),
                        customization: temp,
                    })
                }
            }
            Self::Customize(inner) => Self::Review(inner),
            Self::Dummy
            | Self::Review(_)
            | Self::Flashing(_)
            | Self::FlashingFail(_)
            | Self::FlashingCancel(_)
            | Self::FlashingSuccess(_)
            | Self::AppInfo(_) => {
                panic!("Unexpected message")
            }
        };

        match self {
            Self::ChooseOs(inner) => {
                let subitems_task = inner
                    .common
                    .fetch_remote_subitems(inner.selected_board, &[]);
                let icons_task = inner.common.fetch_os_images(inner.selected_board, &[]);

                Task::batch([subitems_task, icons_task])
            }
            Self::Review(inner) => match inner.customization() {
                helpers::FlashingCustomization::LinuxSdSysconfig(c) => {
                    let mut temp = inner
                        .app_config()
                        .sd_customization()
                        .cloned()
                        .unwrap_or_default();
                    temp.update_sysconfig(c.clone());
                    inner.common.app_config.update_sd_customization(temp);

                    inner.save_app_config()
                }
                helpers::FlashingCustomization::Bcf(c) => {
                    inner.common.app_config.update_bcf_customization(c.clone());

                    inner.save_app_config()
                }
                _ => Task::none(),
            },
            _ => Task::none(),
        }
    }
}
