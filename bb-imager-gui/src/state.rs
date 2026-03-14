use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

use bb_config::config;
use iced::{Task, widget};

use crate::{
    BBImager, constants,
    helpers::{self, DestinationItem, OsImageId, OsImageItem},
    message::BBImagerMessage,
    persistance, updater,
};

#[derive(Debug)]
pub(crate) struct BBImagerCommon {
    pub(crate) app_config: persistance::GuiConfiguration,
    pub(crate) boards: helpers::Boards,
    pub(crate) downloader: bb_downloader::Downloader,
    pub(crate) timezones: widget::combo_box::State<String>,
    pub(crate) keymaps: widget::combo_box::State<String>,

    // Constant image handles
    pub(crate) board_svg_handle: widget::svg::Handle,
    pub(crate) downloading_svg_handle: widget::svg::Handle,
    pub(crate) arrow_forward_svg_handle: widget::svg::Handle,
    pub(crate) format_svg_handle: widget::svg::Handle,
    pub(crate) file_add_svg_handle: widget::svg::Handle,
    pub(crate) arrow_back_svg_handle: widget::svg::Handle,
    pub(crate) usb_svg_handle: widget::svg::Handle,
    pub(crate) file_save_icon: widget::svg::Handle,
    pub(crate) info_svg_handle: widget::svg::Handle,
    pub(crate) copy_svg_handle: widget::svg::Handle,
    pub(crate) window_icon_handle: widget::image::Handle,

    pub(crate) img_handle_cache: helpers::ImageHandleCache,

    pub(crate) scroll_id: widget::Id,
}

impl BBImagerCommon {
    pub(crate) fn updater_task(&self) -> Task<BBImagerMessage> {
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

    pub(crate) fn fetch_images(
        &self,
        iter: impl IntoIterator<Item = url::Url>,
    ) -> Task<BBImagerMessage> {
        let tasks = iter.into_iter().map(|icon| {
            let downloader = self.downloader.clone();
            let icon_clone = icon.clone();
            let icon_clone2 = icon.clone();
            Task::perform(
                async move { downloader.download_no_cache(icon_clone).await },
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

    pub(crate) fn fetch_board_images(&self) -> Task<BBImagerMessage> {
        // Do not try downloading same image multiple times
        let icons: HashSet<url::Url> = self
            .boards
            .devices()
            .filter_map(|(_, dev)| dev.icon.clone())
            .collect();

        self.fetch_images(icons)
    }

    /// Expects image at target is not remote sub item
    fn crawl_image_level(
        &self,
        board: usize,
        target: &[usize],
    ) -> (HashSet<url::Url>, Vec<(Vec<usize>, url::Url)>) {
        let mut icons = HashSet::new();
        let mut remote_imgs = Vec::new();

        let os_images = self
            .boards
            .images(board, target)
            .expect("Cannot be Remote sublist");
        for (id, img) in os_images {
            match img {
                config::OsListItem::Image(os_image) => {
                    icons.insert(os_image.icon.clone());
                }
                config::OsListItem::SubList(os_sub_list) => {
                    let mut temp_target = target.to_vec();

                    temp_target.push(id);
                    icons.insert(os_sub_list.icon.clone());

                    let (temp_icons, mut temp_remote_imgs) =
                        self.crawl_image_level(board, &temp_target);

                    icons.extend(temp_icons);
                    remote_imgs.append(&mut temp_remote_imgs);
                }
                config::OsListItem::RemoteSubList(os_remote_sub_list) => {
                    let mut temp_target = target.to_vec();

                    temp_target.push(id);
                    icons.insert(os_remote_sub_list.icon.clone());

                    remote_imgs.push((temp_target, os_remote_sub_list.subitems_url.clone()));
                }
            }
        }

        (icons, remote_imgs)
    }

    // Try to resolve all images (including remote images in sublists)
    pub(crate) fn resolve_images(&self, board: usize, target: &[usize]) -> Task<BBImagerMessage> {
        let (icons, remote_imgs) = self.crawl_image_level(board, target);

        let icon_task = self.fetch_images(icons);
        let remote_img_task = remote_imgs.into_iter().map(|(id, u)| {
            let downloader = self.downloader.clone();
            Task::perform(
                async move { downloader.download_json_no_cache(u).await },
                move |x| match x {
                    Ok(item) => BBImagerMessage::ResolveRemoteSubitemItem { item, target: id },
                    Err(e) => {
                        tracing::warn!("Failed to download subitems with error {e}");
                        BBImagerMessage::Null
                    }
                },
            )
        });

        Task::batch(remote_img_task.chain([icon_task]))
    }
}

#[derive(Debug)]
pub(crate) struct ChooseBoardState {
    pub(crate) common: BBImagerCommon,
    pub(crate) selected_board: Option<usize>,
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

impl From<ChooseOsState> for ChooseBoardState {
    fn from(value: ChooseOsState) -> Self {
        Self {
            common: value.common,
            selected_board: Some(value.selected_board),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ChooseOsState {
    pub(crate) common: BBImagerCommon,
    pub(crate) selected_board: usize,
    pub(crate) pos: Vec<usize>,
    pub(crate) selected_image: Option<(OsImageId, helpers::BoardImage)>,
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

    pub(crate) fn copy_svg(&self) -> &widget::svg::Handle {
        &self.common.copy_svg_handle
    }

    pub(crate) fn img_json(&self) -> Option<String> {
        let id = &self.selected_image.as_ref()?.0;

        if let OsImageId::Remote(x) = id {
            let img = self.image(x);
            return Some(serde_json::to_string_pretty(&img).expect("Invalid image"));
        }

        None
    }
}

impl From<CustomizeState> for ChooseOsState {
    fn from(value: CustomizeState) -> Self {
        Self {
            common: value.common,
            selected_board: value.selected_board,
            pos: Vec::new(),
            selected_image: Some(value.selected_image),
        }
    }
}

impl From<ChooseDestState> for ChooseOsState {
    fn from(value: ChooseDestState) -> Self {
        Self {
            common: value.common,
            selected_board: value.selected_board,
            pos: Vec::new(),
            selected_image: Some(value.selected_image),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ChooseDestState {
    pub(crate) common: BBImagerCommon,
    pub(crate) selected_board: usize,
    pub(crate) selected_image: (OsImageId, helpers::BoardImage),
    pub(crate) selected_dest: Option<helpers::Destination>,
    pub(crate) destinations: Vec<helpers::Destination>,
    pub(crate) filter_destination: bool,
}

impl ChooseDestState {
    pub(crate) fn destinations<'a>(&'a self) -> impl Iterator<Item = DestinationItem<'a>> + 'a {
        let iter = self.destinations.iter().map(DestinationItem::Destination);

        let temp = match self.selected_image.1.file_name() {
            Some(x) => vec![DestinationItem::SaveToFile(x)],
            None => vec![],
        };

        iter.chain(temp)
    }

    pub(crate) fn usb_svg(&self) -> &widget::svg::Handle {
        &self.common.usb_svg_handle
    }

    pub(crate) fn file_save_icon(&self) -> &widget::svg::Handle {
        &self.common.file_save_icon
    }

    pub(crate) fn selected_board(&self) -> &config::Device {
        self.common.boards.device(self.selected_board)
    }

    pub(crate) fn instruction(&self) -> Option<&str> {
        match self.selected_image.1.info_text() {
            Some(x) => Some(x),
            None => self.selected_board().instructions.as_deref(),
        }
    }
}

impl From<CustomizeState> for ChooseDestState {
    fn from(value: CustomizeState) -> Self {
        Self {
            common: value.common,
            selected_board: value.selected_board,
            selected_image: value.selected_image,
            selected_dest: Some(value.selected_dest),
            destinations: Vec::new(),
            filter_destination: true,
        }
    }
}

#[derive(Debug)]
pub(crate) struct CustomizeState {
    pub(crate) common: BBImagerCommon,
    pub(crate) selected_board: usize,
    pub(crate) selected_image: (OsImageId, helpers::BoardImage),
    pub(crate) selected_dest: helpers::Destination,
    pub(crate) customization: helpers::FlashingCustomization,
}

impl CustomizeState {
    pub(crate) fn timezones(&self) -> &widget::combo_box::State<String> {
        &self.common.timezones
    }

    pub(crate) fn keymaps(&self) -> &widget::combo_box::State<String> {
        &self.common.keymaps
    }

    pub(crate) fn app_config(&self) -> &persistance::GuiConfiguration {
        &self.common.app_config
    }

    pub(crate) fn save_app_config(&self) -> Task<BBImagerMessage> {
        let config = self.app_config().clone();
        Task::future(async move {
            if let Err(e) = config.save().await {
                tracing::error!("Failed to save config: {e}");
            }
            BBImagerMessage::Null
        })
    }

    pub(crate) fn selected_board(&self) -> &str {
        self.common.boards.device(self.selected_board).name.as_str()
    }

    pub(crate) fn selected_image(&self) -> String {
        self.selected_image.1.to_string()
    }

    pub(crate) fn selected_destination(&self) -> String {
        match self.selected_dest.size() {
            Some(x) => format!("{} ({})", self.selected_dest, helpers::pretty_bytes(x)),
            None => self.selected_dest.to_string(),
        }
    }

    pub(crate) fn is_download(&self) -> bool {
        self.selected_dest.is_download_action()
    }

    pub(crate) fn modifications(&self) -> Vec<&'static str> {
        match &self.customization {
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
pub(crate) struct FlashingState {
    pub(crate) common: BBImagerCommon,
    pub(crate) selected_board: usize,
    pub(crate) cancel_flashing: iced::task::Handle,
    pub(crate) progress: bb_flasher::DownloadFlashingStatus,
    pub(crate) start_timestamp: Option<Instant>,
    pub(crate) is_download: bool,
}

impl FlashingState {
    pub(crate) fn selected_board(&self) -> &config::Device {
        self.common.boards.device(self.selected_board)
    }

    pub(crate) fn time_remaining(&self) -> Option<Duration> {
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

    pub(crate) fn progress_update(&mut self, u: bb_flasher::DownloadFlashingStatus) {
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
pub(crate) struct FlashingFinishState {
    pub(crate) common: BBImagerCommon,
    pub(crate) selected_board: usize,
    pub(crate) is_download: bool,
}

impl FlashingFinishState {
    pub(crate) fn selected_board(&self) -> &config::Device {
        self.common.boards.device(self.selected_board)
    }
}

impl From<FlashingState> for FlashingFinishState {
    fn from(value: FlashingState) -> Self {
        Self {
            common: value.common,
            selected_board: value.selected_board,
            is_download: value.is_download,
        }
    }
}

pub(crate) struct FlashingFailState {
    pub(crate) common: BBImagerCommon,
    pub(crate) err: String,
    pub(crate) logs: widget::text_editor::Content,
}

// State for Pages that can be opened from any of the normal pages but are not part of normal flow.
// Eg: Application info
pub(crate) enum OverlayData {
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
    pub(crate) fn common_mut(&mut self) -> &mut BBImagerCommon {
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

    pub(crate) fn common(&self) -> &BBImagerCommon {
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

pub(crate) struct OverlayState {
    pub(crate) page: OverlayData,
    pub(crate) log_path: String,
    pub(crate) license: widget::text_editor::Content,
    pub(crate) cache_dir: String,
}

impl OverlayState {
    pub(crate) fn new(page: OverlayData) -> Self {
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

    pub(crate) fn common(&self) -> &BBImagerCommon {
        self.page.common()
    }

    pub(crate) fn common_mut(&mut self) -> &mut BBImagerCommon {
        self.page.common_mut()
    }
}
