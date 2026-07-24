use std::time::{Duration, Instant};

use bb_config::config;
use iced::{Task, widget};

use crate::{
    BBImager, constants,
    db::{self, Board},
    helpers::{self, DestinationItem, OsImageId, OsImageItem, blocking_future},
    message::BBImagerMessage,
    persistance, updater,
};

#[derive(Debug)]
pub(crate) struct BBImagerCommon {
    pub(crate) app_config: persistance::GuiConfiguration,
    pub(crate) downloader: bb_downloader::Downloader,
    pub(crate) timezones: widget::combo_box::State<String>,
    pub(crate) keymaps: widget::combo_box::State<String>,

    pub(crate) img_handle_cache: bb_iced_widgets::cached_icon::Cache<url::Url>,

    pub(crate) scroll_id: widget::Id,
    pub(crate) db: db::Db,
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

    pub(crate) fn fetch_board_images(&self) -> Task<BBImagerMessage> {
        let db = self.db.clone();
        Task::perform(
            blocking_future(move || db.board_icons().unwrap()),
            BBImagerMessage::FilterResolveImages,
        )
    }
}

#[derive(Debug)]
pub(crate) struct ChooseBoardState {
    pub(crate) common: BBImagerCommon,
    pub(crate) boards: Vec<db::BoardListItem>,
    pub(crate) selected_board: Option<Board>,
    pub(crate) search_text: String,
}

impl ChooseBoardState {
    pub(crate) fn refresh_board_list(&self) -> Task<BBImagerMessage> {
        let db = self.common.db.clone();
        let search = self.search_text.clone();

        Task::perform(
            blocking_future(move || db.board_list(&search).unwrap()),
            BBImagerMessage::UpdateBoardList,
        )
    }

    pub(crate) fn update_search(&mut self, search: String) -> Task<BBImagerMessage> {
        self.search_text = search;
        self.refresh_board_list()
    }
}

impl From<ChooseOsState> for ChooseBoardState {
    fn from(value: ChooseOsState) -> Self {
        Self {
            common: value.common,
            boards: Vec::new(),
            selected_board: Some(value.selected_board),
            search_text: String::new(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ChooseOsState {
    pub(crate) common: BBImagerCommon,
    pub(crate) selected_board: Board,
    pub(crate) images: Vec<OsImageItem>,
    pub(crate) pos: Option<i64>,
    pub(crate) flasher: config::Flasher,
    pub(crate) selected_image: Option<(OsImageId, helpers::BoardImage)>,
    pub(crate) search_text: String,
}

impl ChooseOsState {
    pub(crate) fn update_images(&mut self, mut imgs: Vec<OsImageItem>, pos: Option<i64>) {
        match self.flasher {
            config::Flasher::SdCard => imgs.extend([
                OsImageItem::format("Format SD Card".into()),
                OsImageItem::local(config::Flasher::SdCard),
            ]),
            _ => imgs.push(OsImageItem::local(self.flasher)),
        }

        self.images = imgs;
        self.pos = pos;
    }

    pub(crate) fn img_json(&self) -> Option<String> {
        self.selected_image
            .as_ref()
            .map(|(_, b)| serde_json::to_string_pretty(&b).unwrap())
    }

    pub(crate) fn resolve_remote_sublists(
        &self,
        board_id: i64,
        pos: Option<i64>,
    ) -> Task<BBImagerMessage> {
        let db = self.common.db.clone();
        let downloader = self.common.downloader.clone();

        Task::future(blocking_future(move || {
            db.os_remote_sublists(board_id, pos).unwrap()
        }))
        .then(move |items| helpers::fetch_remote_subitems(items, downloader.clone()))
    }

    pub(crate) fn resolve_all_remote_sublists(&self, board_id: i64) -> Task<BBImagerMessage> {
        let db = self.common.db.clone();
        let downloader = self.common.downloader.clone();

        Task::future(blocking_future(move || {
            db.os_remote_sublists_by_board(board_id).unwrap()
        }))
        .then(move |items| helpers::fetch_remote_subitems(items, downloader.clone()))
    }

    pub(crate) fn refresh_image_list(&self) -> Task<BBImagerMessage> {
        let db = self.common.db.clone();
        let pos = self.pos;
        let board_id = self.selected_board.id;

        if self.search_text.is_empty() {
            Task::perform(
                blocking_future(move || {
                    let imgs = db.os_image_items(board_id, pos).unwrap();
                    (imgs, pos)
                }),
                BBImagerMessage::UpdateOsList,
            )
        } else {
            let search = self.search_text.clone();
            Task::perform(
                blocking_future(move || {
                    let imgs = db.os_images_by_name(board_id, &search).unwrap();
                    (imgs, pos)
                }),
                BBImagerMessage::UpdateOsList,
            )
        }
    }

    pub(crate) fn update_search(&mut self, search: String) -> Task<BBImagerMessage> {
        self.search_text = search;
        self.refresh_image_list()
    }

    pub fn update_pos(
        &mut self,
        pos: Option<i64>,
        flasher: config::Flasher,
    ) -> Task<BBImagerMessage> {
        self.pos = pos;
        self.flasher = flasher;
        self.refresh_image_list()
    }
}

impl From<CustomizeState> for ChooseOsState {
    fn from(value: CustomizeState) -> Self {
        ChooseDestState::from(value).into()
    }
}

impl From<ChooseDestState> for ChooseOsState {
    fn from(value: ChooseDestState) -> Self {
        Self {
            common: value.common,
            images: Vec::new(),
            flasher: value.selected_board.flasher,
            selected_board: value.selected_board,
            pos: None,
            selected_image: Some(value.selected_image),
            search_text: String::new(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ChooseDestState {
    pub(crate) common: BBImagerCommon,
    pub(crate) selected_board: Board,
    pub(crate) selected_image: (OsImageId, helpers::BoardImage),
    pub(crate) selected_dest: Option<helpers::Destination>,
    pub(crate) destinations: Vec<helpers::Destination>,
    pub(crate) filter_destination: bool,
    pub(crate) search_text: String,
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

    pub(crate) fn instruction(&self) -> Option<&str> {
        match self.selected_image.1.info_text() {
            Some(x) => Some(x),
            None => self.selected_board.instructions.as_deref(),
        }
    }

    pub(crate) fn update_search(&mut self, search: String) {
        self.search_text = search;
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
            search_text: String::new(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct CustomizeState {
    pub(crate) common: BBImagerCommon,
    pub(crate) selected_board: Board,
    pub(crate) selected_image: (OsImageId, helpers::BoardImage),
    pub(crate) selected_dest: helpers::Destination,
    pub(crate) customization: helpers::FlashingCustomization,
}

impl CustomizeState {
    pub(crate) fn save_app_config(&self) -> Task<BBImagerMessage> {
        let config = self.common.app_config.clone();
        Task::future(blocking_future(move || {
            if let Err(e) = config.save() {
                tracing::error!("Failed to save config: {e}");
            }
            BBImagerMessage::Null
        }))
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
                let mut ans = helpers::sd_modifications_common(x);
                if x.usb_enable_dhcp == Some(true) {
                    ans.push("• USB DHCP enabled");
                }

                ans
            }
            helpers::FlashingCustomization::LinuxSdCloudInit(x) => {
                helpers::sd_modifications_common(x)
            }
            helpers::FlashingCustomization::Bcf(x) | helpers::FlashingCustomization::Zepto(x) => {
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
    pub(crate) selected_board: Board,
    pub(crate) cancel_flashing: iced::task::Handle,
    pub(crate) progress: bb_flasher::DownloadFlashingStatus,
    pub(crate) start_timestamp: Option<Instant>,
    pub(crate) is_download: bool,
    pub(crate) selected_image: (OsImageId, helpers::BoardImage),
    pub(crate) selected_dest: helpers::Destination,
    pub(crate) customization: helpers::FlashingCustomization,
}

impl FlashingState {
    pub(crate) fn time_remaining(&self) -> Option<Duration> {
        time_remaining_from(self.progress, self.start_timestamp.map(|t| t.elapsed()))
    }

    pub(crate) fn progress_update(&mut self, u: bb_flasher::DownloadFlashingStatus) {
        // Required for better time estimate.
        match u {
            bb_flasher::DownloadFlashingStatus::DownloadingProgress(_)
            | bb_flasher::DownloadFlashingStatus::FlashingProgress(_)
                if self.start_timestamp.is_none() =>
            {
                self.start_timestamp = Some(Instant::now())
            }
            _ => {}
        }

        self.progress = u;
    }
}

/// Estimate the remaining flashing time from the current `progress` and how
/// much time has `elapsed` since the first progress update.
///
/// Split out of [`FlashingState::time_remaining`] so the ETA math is testable
/// without an `Instant` clock: a linear extrapolation `elapsed * (1 - x) / x`,
/// suppressed until progress clears a small threshold to avoid wild early
/// estimates.
fn time_remaining_from(
    progress: bb_flasher::DownloadFlashingStatus,
    elapsed: Option<Duration>,
) -> Option<Duration> {
    const THRESHOLD: f32 = 0.02;

    match progress {
        bb_flasher::DownloadFlashingStatus::FlashingProgress(x)
        | bb_flasher::DownloadFlashingStatus::DownloadingProgress(x) => {
            if x < THRESHOLD {
                None
            } else {
                let t = elapsed?;
                let x = x.clamp(0.0, 1.0);
                let scale = (1.0 - x) / x;
                Some(t.mul_f32(scale))
            }
        }
        bb_flasher::DownloadFlashingStatus::Customizing => Some(Duration::from_secs(1)),
        _ => None,
    }
}

#[derive(Debug)]
pub(crate) struct FlashingFinishState {
    pub(crate) common: BBImagerCommon,
    pub(crate) selected_board: Board,
    pub(crate) is_download: bool,
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
    pub(crate) selected_board: Board,
    pub(crate) selected_image: (OsImageId, helpers::BoardImage),
    pub(crate) selected_dest: helpers::Destination,
    pub(crate) customization: helpers::FlashingCustomization,
}

impl From<FlashingFailState> for CustomizeState {
    fn from(value: FlashingFailState) -> Self {
        Self {
            common: value.common,
            selected_board: value.selected_board,
            selected_image: value.selected_image,
            selected_dest: value.selected_dest,
            customization: value.customization,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::time_remaining_from;
    use bb_flasher::DownloadFlashingStatus;
    use std::time::Duration;

    #[test]
    fn eta_scales_linearly_with_remaining_fraction() {
        // At 50% after 10s, the remaining half should take another ~10s.
        assert_eq!(
            time_remaining_from(
                DownloadFlashingStatus::FlashingProgress(0.5),
                Some(Duration::from_secs(10)),
            ),
            Some(Duration::from_secs(10))
        );
        // At 25% after 10s, the remaining 75% extrapolates to 30s.
        assert_eq!(
            time_remaining_from(
                DownloadFlashingStatus::FlashingProgress(0.25),
                Some(Duration::from_secs(10)),
            ),
            Some(Duration::from_secs(30))
        );
    }

    #[test]
    fn eta_uses_the_same_math_for_downloads() {
        assert_eq!(
            time_remaining_from(
                DownloadFlashingStatus::DownloadingProgress(0.5),
                Some(Duration::from_secs(4)),
            ),
            Some(Duration::from_secs(4))
        );
    }

    #[test]
    fn eta_suppressed_below_threshold() {
        // Below 2% the estimate is too noisy, so no ETA is reported.
        assert_eq!(
            time_remaining_from(
                DownloadFlashingStatus::FlashingProgress(0.01),
                Some(Duration::from_secs(10)),
            ),
            None
        );
    }

    #[test]
    fn eta_requires_a_start_timestamp() {
        // Past the threshold but with no elapsed time recorded yet.
        assert_eq!(
            time_remaining_from(DownloadFlashingStatus::FlashingProgress(0.5), None),
            None
        );
    }

    #[test]
    fn eta_clamps_progress_above_one() {
        // A progress value >1.0 clamps to 1.0, yielding a zero remainder.
        assert_eq!(
            time_remaining_from(
                DownloadFlashingStatus::FlashingProgress(1.5),
                Some(Duration::from_secs(10)),
            ),
            Some(Duration::ZERO)
        );
    }

    #[test]
    fn customizing_reports_fixed_estimate() {
        assert_eq!(
            time_remaining_from(DownloadFlashingStatus::Customizing, None),
            Some(Duration::from_secs(1))
        );
    }

    #[test]
    fn non_progress_states_have_no_eta() {
        assert_eq!(
            time_remaining_from(
                DownloadFlashingStatus::Preparing,
                Some(Duration::from_secs(5))
            ),
            None
        );
        assert_eq!(
            time_remaining_from(
                DownloadFlashingStatus::Verifying,
                Some(Duration::from_secs(5))
            ),
            None
        );
    }
}
