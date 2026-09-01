use std::sync::Arc;
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
    pub(crate) timezones: widget::combo_box::State<chrono_tz::Tz>,
    pub(crate) keymaps: widget::combo_box::State<&'static str>,

    pub(crate) img_handle_cache: bb_iced_widgets::cached_icon::Cache<std::sync::Arc<url::Url>>,

    pub(crate) scroll_id: widget::Id,
    pub(crate) search_id: widget::Id,
    pub(crate) list_selection_id: widget::Id,
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
    pub(crate) boards: Box<[db::BoardListItem]>,
    pub(crate) selected_board: Option<Board>,
    pub(crate) search_text: Arc<str>,
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

    pub(crate) fn update_search(&mut self, search: Arc<str>) -> Task<BBImagerMessage> {
        self.search_text = search;
        self.refresh_board_list()
    }

    pub(crate) fn list_select_relative(&self, delta: i32) -> Option<BBImagerMessage> {
        if self.boards.is_empty() {
            return None;
        }

        let next = list_relative_index(
            self.selected_board
                .as_ref()
                .and_then(|b| self.boards.iter().position(|x| x.id == b.id)),
            self.boards.len(),
            delta,
        );

        Some(BBImagerMessage::SelectBoardById(self.boards[next].id))
    }
}

impl From<ChooseOsState> for ChooseBoardState {
    fn from(value: ChooseOsState) -> Self {
        Self {
            common: value.common,
            boards: Box::default(),
            selected_board: Some(value.selected_board),
            search_text: "".into(),
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
    pub(crate) search_text: Arc<str>,
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

    /// Id of the selected image's config entry, if it has one.
    ///
    /// Local images and the SD format action are not in the config, so there is
    /// nothing to copy for them.
    pub(crate) fn selected_image_config_id(&self) -> Option<i64> {
        match self.selected_image.as_ref()?.0 {
            helpers::OsImageId::OsImage(id) => Some(id),
            _ => None,
        }
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

    pub(crate) fn update_search(&mut self, search: Arc<str>) -> Task<BBImagerMessage> {
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

    pub(crate) fn list_select_relative(&self, delta: i32) -> Option<BBImagerMessage> {
        if self.images.is_empty() {
            return None;
        }

        let next = list_relative_index(
            self.selected_image
                .as_ref()
                .and_then(|(id, _)| self.images.iter().position(|x| x.id == *id)),
            self.images.len(),
            delta,
        );

        Some(BBImagerMessage::SelectOs(self.images[next].id))
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
            search_text: "".into(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ChooseDestState {
    pub(crate) common: BBImagerCommon,
    pub(crate) selected_board: Board,
    pub(crate) selected_image: (OsImageId, helpers::BoardImage),
    pub(crate) selected_dest: Option<helpers::Destination>,
    pub(crate) destinations: Box<[helpers::Destination]>,
    pub(crate) filter_destination: bool,
    pub(crate) search_text: Arc<str>,
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

    pub(crate) fn update_search(&mut self, search: Arc<str>) {
        self.search_text = search;
    }

    pub(crate) fn list_select_relative(&self, delta: i32) -> Option<BBImagerMessage> {
        let items: Vec<BBImagerMessage> = self.destinations().map(|d| d.msg()).collect();
        if items.is_empty() {
            return None;
        }

        let current = self
            .selected_dest
            .as_ref()
            .and_then(|sel| self.destinations().position(|d| d.is_selected(sel)));

        let next = list_relative_index(current, items.len(), delta);
        Some(items[next].clone())
    }
}

/// The choices that make up a flashing job.
///
/// Complete once a destination has been picked, and carried unchanged from
/// there through Customize, Review, Flashing and the failure page.
#[derive(Debug)]
pub(crate) struct FlashingContext {
    pub(crate) selected_board: Board,
    pub(crate) selected_image: (OsImageId, helpers::BoardImage),
    pub(crate) selected_dest: helpers::Destination,
    pub(crate) customization: helpers::FlashingCustomization,
    /// Whether the Customize page is part of this flow.
    ///
    /// Decided once, when the destination is picked, so going back from Review
    /// does not have to ask [`helpers::no_customization`] the same question a
    /// second time and hope it answers consistently.
    pub(crate) has_customization: bool,
}

impl FlashingContext {
    pub(crate) fn selected_destination(&self) -> String {
        match self.selected_dest.size() {
            Some(x) => format!("{} ({})", self.selected_dest, helpers::pretty_bytes(x)),
            None => self.selected_dest.to_string(),
        }
    }

    pub(crate) fn is_download(&self) -> bool {
        self.selected_dest.is_download_action()
    }

    /// Rebuild the destination page this context was completed on.
    pub(crate) fn choose_dest(self, common: BBImagerCommon) -> ChooseDestState {
        ChooseDestState {
            common,
            selected_board: self.selected_board,
            selected_image: self.selected_image,
            selected_dest: Some(self.selected_dest),
            destinations: Box::default(),
            filter_destination: true,
            search_text: "".into(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct CustomizeState {
    pub(crate) common: BBImagerCommon,
    pub(crate) ctx: FlashingContext,
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
}

#[derive(Debug)]
pub(crate) struct FlashingState {
    pub(crate) common: BBImagerCommon,
    pub(crate) ctx: FlashingContext,
    pub(crate) cancel_flashing: iced::task::Handle,
    pub(crate) progress: bb_flasher::DownloadFlashingStatus,
    pub(crate) start_timestamp: Option<Instant>,
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
            is_download: value.ctx.is_download(),
            common: value.common,
            selected_board: value.ctx.selected_board,
        }
    }
}

pub(crate) struct FlashingFailState {
    pub(crate) common: BBImagerCommon,
    pub(crate) ctx: FlashingContext,
    pub(crate) err: String,
    pub(crate) logs: widget::text_editor::Content,
}

impl FlashingFailState {
    pub(crate) fn new(
        state: FlashingState,
        err: String,
        logs: widget::text_editor::Content,
    ) -> Self {
        Self {
            common: state.common,
            ctx: state.ctx,
            err,
            logs,
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

fn list_relative_index(current: Option<usize>, len: usize, delta: i32) -> usize {
    debug_assert!(len > 0);

    let current = current.unwrap_or(if delta >= 0 { len.saturating_sub(1) } else { 0 });
    ((current as i32 + delta).rem_euclid(len as i32)) as usize
}

#[cfg(test)]
mod tests {
    use super::{list_relative_index, time_remaining_from};
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
    fn list_relative_index_wraps_forward() {
        assert_eq!(list_relative_index(Some(2), 5, 1), 3);
        assert_eq!(list_relative_index(Some(4), 5, 1), 0);
    }

    #[test]
    fn list_relative_index_wraps_backward() {
        assert_eq!(list_relative_index(Some(0), 5, -1), 4);
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
