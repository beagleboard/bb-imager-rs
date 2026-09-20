use std::sync::Arc;
use std::time::Instant;

use bb_config::config;
use iced::{Task, widget};

use bb_imager_ui::image_selection::ImageId;

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

    pub(crate) fn refresh_image_icons(&self, board_id: i64) -> Task<BBImagerMessage> {
        let db = self.db.clone();
        Task::perform(
            blocking_future(move || db.os_image_icons_by_board_id(board_id).unwrap()),
            BBImagerMessage::FilterResolveImages,
        )
    }
}

/// The first-run udev notice for sandboxed installs. Static page; only the
/// shared `common` travels through it.
#[derive(Debug)]
pub(crate) struct SandboxNoticeState {
    pub(crate) common: BBImagerCommon,
    pub(crate) state: bb_imager_ui::sandbox_notice::State,
}

impl SandboxNoticeState {
    pub(crate) fn new(common: BBImagerCommon) -> Self {
        Self {
            common,
            state: bb_imager_ui::sandbox_notice::State::new(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ChooseBoardState {
    pub(crate) common: BBImagerCommon,
    /// The full board, kept for the rest of the flow (`flasher`, `instructions`,
    /// `bootfs`), none of which the page renders.
    pub(crate) selected_board: Option<Board>,
    pub(crate) state: bb_imager_ui::board_selection::State,
}

impl ChooseBoardState {
    pub(crate) fn new(common: BBImagerCommon) -> Self {
        Self {
            common,
            selected_board: None,
            state: Default::default(),
        }
    }

    /// Record `board` as the selection, both for the flow and for the page.
    pub(crate) fn select_board(&mut self, board: Board) {
        self.state.selected = Some((&board).into());
        self.selected_board = Some(board);
    }

    pub(crate) fn refresh_board_list(&self) -> Task<BBImagerMessage> {
        let db = self.common.db.clone();
        let search = self.state.search.clone();

        Task::perform(
            blocking_future(move || db.board_list(&search).unwrap()),
            BBImagerMessage::UpdateBoardList,
        )
    }

    pub(crate) fn update_search(&mut self, search: Arc<str>) -> Task<BBImagerMessage> {
        self.state.search = search;
        self.refresh_board_list()
    }
}

impl From<ChooseOsState> for ChooseBoardState {
    fn from(value: ChooseOsState) -> Self {
        let mut res = Self::new(value.common);
        res.select_board(value.selected_board);
        res
    }
}

impl From<&Board> for bb_imager_ui::board_selection::BoardDetails {
    fn from(value: &Board) -> Self {
        Self {
            id: value.id,
            name: value.name.clone(),
            icon: value.icon.clone(),
            description: value.description.clone().into(),
            specification: value.specification.clone().into(),
            documentation: value.documentation.clone(),
            // The page renders a plain link, so the OSHWA id is resolved here
            // where the parse can still fail quietly.
            oshw: value.oshw.as_ref().and_then(|x| {
                url::Url::parse(&format!("{}/{}.html", constants::OSHW_BASE_URL, x)).ok()
            }),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ChooseOsState {
    pub(crate) common: BBImagerCommon,
    pub(crate) selected_board: Board,
    pub(crate) flasher: config::Flasher,
    /// Carries the flasher machinery the page cannot render; the page keeps its
    /// own renderable projection in `state.selected`.
    pub(crate) selected_image: Option<(ImageId, helpers::BoardImage)>,
    pub(crate) state: bb_imager_ui::image_selection::State,
}

impl ChooseOsState {
    pub(crate) fn update_images(
        &mut self,
        mut imgs: Vec<bb_imager_ui::image_selection::ImageItem>,
        pos: Option<i64>,
    ) {
        if self.flasher == config::Flasher::SdCard {
            imgs.push(bb_imager_ui::image_selection::ImageItem {
                id: ImageId::Format,
                icon: None,
                label: "Format SD Card".into(),
            });
        }

        imgs.push(bb_imager_ui::image_selection::ImageItem {
            id: ImageId::Local(self.flasher),
            icon: None,
            label: "Select Local Image".into(),
        });

        self.state.images = imgs.into();
        self.state.pos = pos;
    }

    /// Record `img` as the selection, both for the flow and for the page.
    pub(crate) fn select_image(&mut self, id: ImageId, img: helpers::BoardImage) {
        self.state.selected = Some(helpers::image_details(id, &img));
        self.selected_image = Some((id, img));
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
        let pos = self.state.pos;
        let board_id = self.selected_board.id;

        if self.state.search.is_empty() {
            Task::perform(
                blocking_future(move || {
                    let imgs = db.os_image_items(board_id, pos).unwrap();
                    (imgs, pos)
                }),
                BBImagerMessage::UpdateOsList,
            )
        } else {
            let search = self.state.search.clone();
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
        self.state.search = search;
        self.refresh_image_list()
    }

    pub fn update_pos(
        &mut self,
        pos: Option<i64>,
        flasher: config::Flasher,
    ) -> Task<BBImagerMessage> {
        self.state.pos = pos;
        self.flasher = flasher;
        self.refresh_image_list()
    }
}

impl From<ChooseDestState> for ChooseOsState {
    fn from(value: ChooseDestState) -> Self {
        let mut res = Self {
            common: value.common,
            flasher: value.selected_board.flasher,
            selected_board: value.selected_board,
            selected_image: None,
            state: Default::default(),
        };
        let (id, img) = value.selected_image;
        res.select_image(id, img);
        res
    }
}

#[derive(Debug)]
pub(crate) struct ChooseDestState {
    pub(crate) common: BBImagerCommon,
    pub(crate) selected_board: Board,
    pub(crate) selected_image: (ImageId, helpers::BoardImage),
    /// Carries the flasher machinery the page cannot render.
    pub(crate) selected_dest: Option<helpers::Destination>,
    /// Kept so a [`DestId`] can be resolved back to a real destination, and so
    /// the 1 Hz refresh can skip redraws when nothing changed.
    pub(crate) destinations: Box<[helpers::Destination]>,
    pub(crate) state: bb_imager_ui::destination_selection::State,
}

impl ChooseDestState {
    pub(crate) fn new(
        common: BBImagerCommon,
        selected_board: Board,
        selected_image: (ImageId, helpers::BoardImage),
    ) -> Self {
        // The image's own note wins over the board-wide one.
        let instruction = match selected_image.1.info_text() {
            Some(x) => Some(x.into()),
            None => selected_board.instructions.clone(),
        };

        // `Some` only for images with something to write out, which is what
        // offers the "Save To File" row.
        let image_file_name = selected_image
            .1
            .file_name()
            .map(|x| helpers::normalize_file_dest(&x).into());

        Self {
            common,
            selected_board,
            selected_image,
            selected_dest: None,
            destinations: Box::default(),
            state: bb_imager_ui::destination_selection::State {
                instruction,
                image_file_name,
                ..Default::default()
            },
        }
    }

    /// Rebuild the page's device rows. The "Save To File" row is derived by the
    /// page from `image_file_name`, so it is unaffected by this.
    pub(crate) fn update_destinations(&mut self, destinations: Box<[helpers::Destination]>) {
        self.state.destinations = destinations.iter().map(helpers::dest_item).collect();
        self.destinations = destinations;
    }

    /// Record `dest` as the selection, both for the flow and for the page.
    pub(crate) fn select_dest(&mut self, dest: helpers::Destination) {
        self.state.selected = Some(helpers::dest_details(&dest));
        self.selected_dest = Some(dest);
    }

    pub(crate) fn update_search(&mut self, search: Arc<str>) {
        self.state.search = search;
    }
}

/// The choices that make up a flashing job.
///
/// Complete once a destination has been picked, and carried unchanged from
/// there through Customize, Review, Flashing and the failure page.
#[derive(Debug)]
pub(crate) struct FlashingContext {
    pub(crate) selected_board: Board,
    pub(crate) selected_image: (ImageId, helpers::BoardImage),
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
        let mut res = ChooseDestState::new(common, self.selected_board, self.selected_image);
        res.select_dest(self.selected_dest);
        res
    }
}

#[derive(Debug)]
pub(crate) struct CustomizeState {
    pub(crate) common: BBImagerCommon,
    pub(crate) ctx: FlashingContext,
    pub(crate) state: Box<bb_imager_ui::configuration::State>,
}

impl CustomizeState {
    pub(crate) fn new(common: BBImagerCommon, ctx: FlashingContext) -> Self {
        Self {
            common,
            state: Box::new(bb_imager_ui::configuration::State {
                customization: ctx.customization.clone().into(),
                default_username: helpers::default_user(),
                default_timezone: helpers::system_timezone(),
                default_keymap: helpers::system_keymap(),
                timezones: widget::combo_box::State::new(chrono_tz::TZ_VARIANTS.to_vec()),
                keymaps: widget::combo_box::State::new(constants::KEYMAP_LAYOUTS.to_vec()),
            }),
            ctx,
        }
    }

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

impl From<ReviewState> for CustomizeState {
    fn from(value: ReviewState) -> Self {
        Self::new(value.common, value.ctx)
    }
}

#[derive(Debug)]
pub(crate) struct ReviewState {
    pub(crate) common: BBImagerCommon,
    pub(crate) ctx: FlashingContext,
    pub(crate) state: bb_imager_ui::review::State,
}

impl ReviewState {
    pub(crate) fn new(common: BBImagerCommon, ctx: FlashingContext) -> Self {
        Self {
            common,
            state: bb_imager_ui::review::State {
                is_download: ctx.is_download(),
                board: ctx.selected_board.name.clone(),
                image: ctx.selected_image.1.to_string().into(),
                destination: ctx.selected_destination().into(),
                modifications: ctx.customization.modifications(),
            },
            ctx,
        }
    }
}

#[derive(Debug)]
pub(crate) struct FlashingState {
    pub(crate) common: BBImagerCommon,
    pub(crate) ctx: FlashingContext,
    pub(crate) cancel_flashing: iced::task::Handle,
    pub(crate) state: Box<bb_imager_ui::flashing::State>,
}

impl FlashingState {
    pub(crate) fn progress_update(&mut self, u: bb_flasher::DownloadFlashingStatus) {
        // Required for better time estimate.
        match u {
            bb_flasher::DownloadFlashingStatus::DownloadingProgress(_)
            | bb_flasher::DownloadFlashingStatus::FlashingProgress(_)
                if self.state.start_timestamp.is_none() =>
            {
                self.state.start_timestamp = Some(Instant::now())
            }
            _ => {}
        }

        self.state.progress = progress_from(u);
    }
}

/// Both types are foreign to this crate, so this cannot be a `From` impl.
fn progress_from(value: bb_flasher::DownloadFlashingStatus) -> bb_imager_ui::flashing::Progress {
    use bb_imager_ui::flashing::Progress;

    match value {
        bb_flasher::DownloadFlashingStatus::Preparing => Progress::Preparing,
        bb_flasher::DownloadFlashingStatus::DownloadingProgress(x) => Progress::Downloading(x),
        bb_flasher::DownloadFlashingStatus::FlashingProgress(x) => Progress::Flashing(x),
        bb_flasher::DownloadFlashingStatus::Verifying => Progress::Verifying,
        bb_flasher::DownloadFlashingStatus::Customizing => Progress::Customizing,
    }
}

#[derive(Debug)]
pub(crate) struct FlashingSuccessState {
    pub(crate) common: BBImagerCommon,
    pub(crate) state: bb_imager_ui::flash_success::State,
}

impl From<FlashingState> for FlashingSuccessState {
    fn from(value: FlashingState) -> Self {
        Self {
            state: bb_imager_ui::flash_success::State {
                is_download: value.ctx.is_download(),
                board: (&value.ctx.selected_board).into(),
            },
            common: value.common,
        }
    }
}

#[derive(Debug)]
pub(crate) struct FlashingCancelState {
    pub(crate) common: BBImagerCommon,
    pub(crate) state: bb_imager_ui::flash_cancel::State,
}

impl From<FlashingState> for FlashingCancelState {
    fn from(value: FlashingState) -> Self {
        Self {
            state: bb_imager_ui::flash_cancel::State {
                board: (&value.ctx.selected_board).into(),
            },
            common: value.common,
        }
    }
}

pub(crate) struct FlashingFailState {
    pub(crate) common: BBImagerCommon,
    pub(crate) ctx: FlashingContext,
    pub(crate) state: bb_imager_ui::flash_fail::State,
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
            state: bb_imager_ui::flash_fail::State {
                reason: err.into(),
                logs,
            },
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
            BBImager::Dummy | BBImager::AppInfo(_) | BBImager::SandboxNotice(_) => Err(()),
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
    pub(crate) state: bb_imager_ui::app_info::State,
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
            state: bb_imager_ui::app_info::State {
                app_name: constants::APP_NAME,
                app_release: constants::APP_RELEASE,
                app_desc: constants::APP_DESC,
                license,
                // TODO: Make Arc
                cache_dir: cache_dir.into(),
                // TODO: Make Arc
                log_path: log_path.into(),
            },
        }
    }

    pub(crate) fn common(&self) -> &BBImagerCommon {
        self.page.common()
    }

    pub(crate) fn common_mut(&mut self) -> &mut BBImagerCommon {
        self.page.common_mut()
    }
}
