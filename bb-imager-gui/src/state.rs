use std::time::Instant;

use bb_config::config;
use bb_imager_ui::dest_selection::Destination as _;
use iced::{Task, widget};

use crate::{
    BBImager, constants,
    db::{self, Board},
    helpers::{self, blocking_future},
    message::BBImagerMessage,
    persistance, updater,
};

#[derive(Debug)]
pub(crate) struct BBImagerCommon {
    pub(crate) app_config: persistance::GuiConfiguration,
    pub(crate) downloader: bb_downloader::Downloader,
    pub(crate) img_handle_cache: bb_iced_widgets::cached_icon::Cache<std::sync::Arc<url::Url>>,
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
    pub(crate) inner: bb_imager_ui::board_selection::State,
}

impl ChooseBoardState {
    pub(crate) fn new(common: BBImagerCommon) -> Self {
        Self {
            common,
            inner: Default::default(),
        }
    }

    pub(crate) fn refresh_board_list(&self) -> Task<BBImagerMessage> {
        let db = self.common.db.clone();
        let search = self.inner.search.clone();

        Task::perform(
            blocking_future(move || db.board_list(&search).unwrap()),
            BBImagerMessage::UpdateBoardList,
        )
    }

    pub(crate) fn update_search(&mut self, search: std::sync::Arc<str>) -> Task<BBImagerMessage> {
        self.inner.search = search;
        self.refresh_board_list()
    }
}

#[derive(Debug)]
pub(crate) struct ChooseOsState {
    pub(crate) common: BBImagerCommon,
    pub(crate) selected_board: Board,
    pub(crate) flasher: config::Flasher,
    pub(crate) inner: bb_imager_ui::img_selection::State,
}

impl ChooseOsState {
    pub(crate) fn update_images(
        &mut self,
        mut imgs: Vec<bb_imager_ui::img_selection::ImageItem>,
        pos: Option<i64>,
    ) {
        if self.flasher == config::Flasher::SdCard {
            imgs.push(bb_imager_ui::img_selection::ImageItem {
                id: bb_imager_ui::img_selection::ImageId::Format,
                label: "Format SD Card".into(),
                icon: None,
                description: "".into(),
                size: None,
                release_date: None,
            })
        }

        imgs.push(bb_imager_ui::img_selection::ImageItem {
            id: bb_imager_ui::img_selection::ImageId::Local(self.flasher),
            label: "Select Local Image".into(),
            icon: None,
            description: "".into(),
            size: None,
            release_date: None,
        });

        self.inner.imgs = imgs.into();
        self.inner.pos = pos;
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
        let pos = self.inner.pos;
        let board_id = self.selected_board.id;

        if self.inner.search.is_empty() {
            Task::perform(
                blocking_future(move || {
                    let imgs = db.os_image_items(board_id, pos).unwrap();
                    (imgs, pos)
                }),
                BBImagerMessage::UpdateOsList,
            )
        } else {
            let search = self.inner.search.clone();
            Task::perform(
                blocking_future(move || {
                    let imgs = db.os_images_by_name(board_id, &search).unwrap();
                    (imgs, pos)
                }),
                BBImagerMessage::UpdateOsList,
            )
        }
    }

    pub(crate) fn refresh_image_icons(&self, board_id: i64) -> Task<BBImagerMessage> {
        let db = self.common.db.clone();
        Task::perform(
            blocking_future(move || db.os_image_icons_by_board_id(board_id).unwrap()),
            BBImagerMessage::FilterResolveImages,
        )
    }

    pub(crate) fn update_search(&mut self, search: std::sync::Arc<str>) -> Task<BBImagerMessage> {
        self.inner.search = search;
        self.refresh_image_list()
    }

    pub fn update_pos(
        &mut self,
        pos: Option<i64>,
        flasher: config::Flasher,
    ) -> Task<BBImagerMessage> {
        self.inner.pos = pos;
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
            flasher: value.selected_board.flasher,
            selected_board: value.selected_board,
            inner: Default::default(),
        }
    }
}

impl From<ReviewState> for ChooseOsState {
    fn from(value: ReviewState) -> Self {
        CustomizeState::from(value).into()
    }
}

#[derive(Debug)]
pub(crate) struct ChooseDestState {
    pub(crate) common: BBImagerCommon,
    pub(crate) selected_board: Board,
    pub(crate) selected_image: helpers::BoardImage,
    pub(crate) inner: bb_imager_ui::dest_selection::State<helpers::Destination>,
}

impl ChooseDestState {
    pub(crate) fn new(
        common: BBImagerCommon,
        selected_board: Board,
        selected_image: helpers::BoardImage,
    ) -> Self {
        // The image's own note wins over the board-wide one.
        let instructions = selected_image
            .info_text()
            .or(selected_board.instructions.as_deref())
            .unwrap_or_default()
            .into();

        // Only offer "Save To File" for images that have something to write out.
        let image_file_name = selected_image
            .file_name()
            .map(|x| helpers::normalize_file_dest(&x).into());

        Self {
            common,
            selected_board,
            selected_image,
            inner: bb_imager_ui::dest_selection::State {
                instructions,
                image_file_name,
                ..Default::default()
            },
        }
    }

    pub(crate) fn update_search(&mut self, search: std::sync::Arc<str>) {
        self.inner.search = search;
    }
}

impl From<CustomizeState> for ChooseDestState {
    fn from(value: CustomizeState) -> Self {
        Self::new(value.common, value.selected_board, value.selected_image)
    }
}

impl From<ReviewState> for ChooseDestState {
    fn from(value: ReviewState) -> Self {
        CustomizeState::from(value).into()
    }
}

#[derive(Debug)]
pub(crate) struct CustomizeState {
    pub(crate) common: BBImagerCommon,
    pub(crate) selected_board: Board,
    pub(crate) selected_image: helpers::BoardImage,
    pub(crate) selected_dest: helpers::Destination,
    pub(crate) inner: bb_imager_ui::customization::State,
}

impl From<ReviewState> for CustomizeState {
    fn from(value: ReviewState) -> Self {
        value.ctx.customize(value.common)
    }
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

/// Everything the user picked over the course of the flow.
///
/// Carried unchanged from [`ReviewState`] through flashing and on to the
/// finish pages, so their sidebars can rebuild any earlier page on demand.
#[derive(Debug)]
pub(crate) struct FlashingContext {
    pub(crate) selected_board: Board,
    pub(crate) selected_image: helpers::BoardImage,
    pub(crate) selected_dest: helpers::Destination,
    pub(crate) customization: helpers::FlashingCustomization,
    /// Whether the customization page is part of this flow. Images with no
    /// supported init format skip straight from destination to review.
    pub(crate) has_customization: bool,
}

impl FlashingContext {
    /// Writing to a file rather than to a device.
    pub(crate) fn is_download(&self) -> bool {
        self.selected_dest.is_download_action()
    }

    fn device_name(&self) -> Box<str> {
        self.selected_board.name.clone()
    }

    fn software_name(&self) -> Box<str> {
        self.selected_image.to_string().into()
    }

    fn storage(&self) -> (Box<str>, Option<u64>) {
        (
            self.selected_dest.to_string().into(),
            self.selected_dest.size(),
        )
    }

    pub(crate) fn review_page(&self) -> bb_imager_ui::review::State {
        let modificiations = self.customization.modifications();

        bb_imager_ui::review::State {
            has_customization: self.has_customization,
            device_name: self.device_name(),
            software_name: self.software_name(),
            storage: self.storage(),
            modificiations,
        }
    }

    pub(crate) fn success_page(&self) -> bb_imager_ui::flash_success::State {
        let modificiations = self.customization.modifications();

        bb_imager_ui::flash_success::State {
            has_customization: self.has_customization,
            device_name: self.device_name(),
            software_name: self.software_name(),
            storage: self.storage(),
            modificiations: modificiations.into_vec(),
        }
    }

    pub(crate) fn choose_os(self, common: BBImagerCommon) -> ChooseOsState {
        ChooseOsState {
            common,
            flasher: self.selected_board.flasher,
            selected_board: self.selected_board,
            inner: Default::default(),
        }
    }

    pub(crate) fn choose_dest(self, common: BBImagerCommon) -> ChooseDestState {
        ChooseDestState::new(common, self.selected_board, self.selected_image)
    }

    pub(crate) fn customize(self, common: BBImagerCommon) -> CustomizeState {
        CustomizeState {
            common,
            selected_board: self.selected_board,
            selected_image: self.selected_image,
            selected_dest: self.selected_dest,
            inner: bb_imager_ui::customization::State {
                customization: self.customization.into(),
                default_username: helpers::default_user(),
                default_timezone: helpers::system_timezone(),
                default_keymap: helpers::system_keymap(),
                timezones: iced::widget::combo_box::State::new(chrono_tz::TZ_VARIANTS.to_vec()),
                keymaps: iced::widget::combo_box::State::new(
                    crate::constants::KEYMAP_LAYOUTS.to_vec(),
                ),
            },
        }
    }

    pub(crate) fn review(self, common: BBImagerCommon) -> ReviewState {
        ReviewState {
            common,
            inner: self.review_page(),
            ctx: self,
        }
    }
}

#[derive(Debug)]
pub(crate) struct ReviewState {
    pub(crate) common: BBImagerCommon,
    pub(crate) ctx: FlashingContext,
    pub(crate) inner: bb_imager_ui::review::State,
}

#[derive(Debug)]
pub(crate) struct FlashingState {
    pub(crate) common: BBImagerCommon,
    pub(crate) ctx: FlashingContext,
    pub(crate) cancel_flashing: iced::task::Handle,
    pub(crate) inner: bb_imager_ui::flashing::State,
}

impl FlashingState {
    pub(crate) fn progress_update(&mut self, u: bb_flasher::DownloadFlashingStatus) {
        // Required for better time estimate.
        match u {
            bb_flasher::DownloadFlashingStatus::DownloadingProgress(_)
            | bb_flasher::DownloadFlashingStatus::FlashingProgress(_)
                if self.inner.start_timestamp.is_none() =>
            {
                self.inner.start_timestamp = Some(Instant::now())
            }
            _ => {}
        }

        self.inner.progress = progress_from(u);
    }
}

/// The UI collapses downloading into writing, since both are just bytes moving
/// towards the destination from the user's point of view.
fn progress_from(u: bb_flasher::DownloadFlashingStatus) -> bb_imager_ui::flashing::Progress {
    match u {
        bb_flasher::DownloadFlashingStatus::Preparing => {
            bb_imager_ui::flashing::Progress::Preparing
        }
        bb_flasher::DownloadFlashingStatus::DownloadingProgress(x)
        | bb_flasher::DownloadFlashingStatus::FlashingProgress(x) => {
            bb_imager_ui::flashing::Progress::Writing(x)
        }
        bb_flasher::DownloadFlashingStatus::Verifying => {
            bb_imager_ui::flashing::Progress::Verifying
        }
        bb_flasher::DownloadFlashingStatus::Customizing => {
            bb_imager_ui::flashing::Progress::Customizing
        }
    }
}

pub(crate) struct FlashingCancelState {
    pub(crate) common: BBImagerCommon,
    pub(crate) ctx: FlashingContext,
    pub(crate) inner: bb_imager_ui::flash_cancel::State,
}

impl From<FlashingState> for FlashingCancelState {
    fn from(value: FlashingState) -> Self {
        Self {
            common: value.common,
            inner: bb_imager_ui::flash_cancel::State {
                has_customization: value.ctx.has_customization,
            },
            ctx: value.ctx,
        }
    }
}

pub(crate) struct FlashingSuccessState {
    pub(crate) common: BBImagerCommon,
    pub(crate) ctx: FlashingContext,
    pub(crate) inner: bb_imager_ui::flash_success::State,
}

impl From<FlashingState> for FlashingSuccessState {
    fn from(value: FlashingState) -> Self {
        Self {
            common: value.common,
            inner: value.ctx.success_page(),
            ctx: value.ctx,
        }
    }
}

pub(crate) struct FlashingFailState {
    pub(crate) common: BBImagerCommon,
    pub(crate) ctx: FlashingContext,
    pub(crate) inner: bb_imager_ui::flash_fail::State,
}

impl FlashingFailState {
    pub(crate) fn new(
        common: BBImagerCommon,
        ctx: FlashingContext,
        err: String,
        logs: widget::text_editor::Content,
    ) -> Self {
        Self {
            common,
            inner: bb_imager_ui::flash_fail::State {
                has_customization: ctx.has_customization,
                reason: err.into(),
                logs,
            },
            ctx,
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
    Review(ReviewState),
    Flashing(FlashingState),
    FlashingCancel(FlashingCancelState),
    FlashingFail(FlashingFailState),
    FlashingSuccess(FlashingSuccessState),
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

    /// Which sidebar entries the app-options page should offer to return to.
    fn previous_page(&self) -> bb_imager_ui::app_options::PreviousPage {
        use bb_imager_ui::app_options::PreviousPage;

        match self {
            Self::ChooseBoard(_) => PreviousPage::Device,
            Self::ChooseOs(_) => PreviousPage::Software,
            Self::ChooseDest(_) => PreviousPage::Destination,
            Self::Customize(_) => PreviousPage::Customization,
            Self::Review(x) => PreviousPage::Review {
                has_customization: x.ctx.has_customization,
            },
            Self::Flashing(x) => PreviousPage::Flashing {
                has_customization: x.ctx.has_customization,
            },
            Self::FlashingCancel(x) => PreviousPage::Flashing {
                has_customization: x.ctx.has_customization,
            },
            Self::FlashingFail(x) => PreviousPage::Flashing {
                has_customization: x.ctx.has_customization,
            },
            Self::FlashingSuccess(x) => PreviousPage::Flashing {
                has_customization: x.ctx.has_customization,
            },
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
            // The overlay cannot wrap itself, and `Dummy` is never observable.
            BBImager::AppInfo(_) | BBImager::Dummy => Err(()),
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
    pub(crate) inner: bb_imager_ui::app_options::State,
}

impl OverlayState {
    pub(crate) fn new(page: OverlayData) -> Self {
        let log_file = helpers::log_file_path().to_string_lossy().into();
        let license = widget::text_editor::Content::with_text(constants::APP_LINCESE);
        let cache_dir = helpers::project_dirs()
            .unwrap()
            .cache_dir()
            .to_string_lossy()
            .into();

        let inner = bb_imager_ui::app_options::State {
            previous_page: page.previous_page(),
            cache_dir,
            log_file,
            license,
        };

        Self { page, inner }
    }

    pub(crate) fn common(&self) -> &BBImagerCommon {
        self.page.common()
    }

    pub(crate) fn common_mut(&mut self) -> &mut BBImagerCommon {
        self.page.common_mut()
    }
}
