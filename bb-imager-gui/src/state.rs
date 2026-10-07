use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use bb_config::config;
use iced::{Task, widget};

use bb_imager_ui::SidebarEntry;
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
}

impl SandboxNoticeState {
    pub(crate) fn new(common: BBImagerCommon) -> Self {
        Self { common }
    }
}

#[derive(Debug)]
pub(crate) struct ChooseBoardState {
    pub(crate) common: BBImagerCommon,
    pub(crate) state: bb_imager_ui::board_selection::State,
}

impl ChooseBoardState {
    pub(crate) fn new(common: BBImagerCommon) -> Self {
        Self {
            common,
            state: Default::default(),
        }
    }

    /// Record `board` as the selection, both for the flow and for the page.
    pub(crate) fn select_board(&mut self, board: Board) {
        self.state.selected = Some(board.into());
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
        Self::new(value.common)
    }
}

impl From<Board> for bb_imager_ui::board_selection::BoardDetails {
    fn from(value: Board) -> Self {
        let mut btns = Vec::with_capacity(2);
        if let Some(x) = value.documentation.as_ref() {
            btns.push(("Documentation", x.clone()));
        }
        if let Some(x) = value.oshw.as_ref() {
            btns.push((
                "OSHW",
                url::Url::parse(&format!("{}/{x}.html", constants::OSHW_BASE_URL)).unwrap(),
            ));
        }

        Self {
            id: value.id,
            name: value.name,
            icon: value.icon,
            description: value.description,
            specification: value.specification.into(),
            buttons: btns.into(),
            flasher: value.flasher,
            instructions: value.instructions,
        }
    }
}

#[derive(Debug)]
pub(crate) struct ChooseOsState {
    pub(crate) common: BBImagerCommon,
    pub(crate) selected_board: helpers::SelectedBoard,
    pub(crate) flasher: config::Flasher,
    pub(crate) state: bb_imager_ui::image_selection::State,
}

impl ChooseOsState {
    pub(crate) fn new(common: BBImagerCommon, selected_board: helpers::SelectedBoard) -> Self {
        Self {
            common,
            flasher: selected_board.flasher,
            selected_board,
            state: Default::default(),
        }
    }

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

    pub(crate) fn select_format_image(&mut self) {
        self.state.selected = Some(bb_imager_ui::image_selection::ImageDetails::Format);
    }

    pub(crate) fn select_local_image(
        &mut self,
        path: Box<Path>,
        flasher: config::Flasher,
        size: u64,
    ) {
        self.state.selected = Some(bb_imager_ui::image_selection::ImageDetails::Local {
            flasher,
            path,
            size,
            init_format: config::InitFormat::None,
        })
    }

    pub(crate) fn select_remote_image(
        &mut self,
        image: crate::db::OsImage,
        flasher: config::Flasher,
    ) {
        let mut buttons = Vec::with_capacity(2);

        if let Some(x) = image.support {
            buttons.push(("Support", x));
        }
        if let Some(x) = image.sbom {
            buttons.push(("SBOM", x));
        }

        let details = [
            ("Release Date", image.release_date.to_string().into()),
            (
                "Image Size",
                helpers::pretty_bytes(image.extract_size as u64).into(),
            ),
            (
                "Download Size",
                helpers::pretty_bytes(image.image_download_size as u64).into(),
            ),
        ]
        .into();

        self.state.selected = Some(bb_imager_ui::image_selection::ImageDetails::Remote {
            id: image.id,
            icon: image.icon,
            title: image.name,
            description: image.description,
            details,
            init_format: image.init_format,
            buttons: buttons.into(),
            flasher,
            info_text: image.info_text,
            file_name: image
                .url
                .path_segments()
                .unwrap()
                .next_back()
                .unwrap()
                .into(),
        })
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
        Self::new(value.common, value.selected_board)
    }
}

impl From<CustomizeState> for ChooseOsState {
    fn from(value: CustomizeState) -> Self {
        Self::new(value.common, value.ctx.selected_board)
    }
}

impl From<ReviewState> for ChooseOsState {
    fn from(value: ReviewState) -> Self {
        Self::new(value.common, value.ctx.selected_board)
    }
}

impl From<FlashingFailState> for ChooseOsState {
    fn from(value: FlashingFailState) -> Self {
        Self::new(value.common, value.ctx.selected_board)
    }
}

impl From<FlashingSuccessState> for ChooseOsState {
    fn from(value: FlashingSuccessState) -> Self {
        Self::new(value.common, value.ctx.selected_board)
    }
}

impl From<FlashingCancelState> for ChooseOsState {
    fn from(value: FlashingCancelState) -> Self {
        Self::new(value.common, value.ctx.selected_board)
    }
}

#[derive(Debug)]
pub(crate) struct ChooseDestState {
    pub(crate) common: BBImagerCommon,
    pub(crate) selected_board: helpers::SelectedBoard,
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
        selected_board: helpers::SelectedBoard,
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

impl From<CustomizeState> for ChooseDestState {
    fn from(value: CustomizeState) -> Self {
        let mut res = ChooseDestState::new(
            value.common,
            value.ctx.selected_board,
            value.ctx.selected_image,
        );
        res.select_dest(value.ctx.selected_dest);
        res
    }
}

impl From<ReviewState> for ChooseDestState {
    fn from(value: ReviewState) -> Self {
        let mut res = ChooseDestState::new(
            value.common,
            value.ctx.selected_board,
            value.ctx.selected_image,
        );
        res.select_dest(value.ctx.selected_dest);
        res
    }
}

impl From<FlashingFailState> for ChooseDestState {
    fn from(value: FlashingFailState) -> Self {
        ReviewState::from(value).into()
    }
}

impl From<FlashingSuccessState> for ChooseDestState {
    fn from(value: FlashingSuccessState) -> Self {
        ReviewState::from(value).into()
    }
}

impl From<FlashingCancelState> for ChooseDestState {
    fn from(value: FlashingCancelState) -> Self {
        ReviewState::from(value).into()
    }
}

/// The choices that make up a flashing job.
///
/// Complete once a destination has been picked, and carried unchanged from
/// there through Customize, Review, Flashing and the failure page.
#[derive(Debug)]
pub(crate) struct FlashingContext {
    pub(crate) selected_board: helpers::SelectedBoard,
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

impl From<FlashingFailState> for CustomizeState {
    fn from(value: FlashingFailState) -> Self {
        Self::new(value.common, value.ctx)
    }
}

impl From<FlashingSuccessState> for CustomizeState {
    fn from(value: FlashingSuccessState) -> Self {
        Self::new(value.common, value.ctx)
    }
}

impl From<FlashingCancelState> for CustomizeState {
    fn from(value: FlashingCancelState) -> Self {
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
                has_customization: ctx.has_customization,
            },
            ctx,
        }
    }
}

impl From<FlashingFailState> for ReviewState {
    fn from(value: FlashingFailState) -> Self {
        ReviewState::new(value.common, value.ctx)
    }
}

impl From<FlashingSuccessState> for ReviewState {
    fn from(value: FlashingSuccessState) -> Self {
        ReviewState::new(value.common, value.ctx)
    }
}

impl From<FlashingCancelState> for ReviewState {
    fn from(value: FlashingCancelState) -> Self {
        ReviewState::new(value.common, value.ctx)
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
    pub(crate) ctx: FlashingContext,
    pub(crate) state: bb_imager_ui::flash_success::State,
}

impl From<FlashingState> for FlashingSuccessState {
    fn from(value: FlashingState) -> Self {
        Self {
            state: bb_imager_ui::flash_success::State {
                board: value.ctx.selected_board.name.clone(),
                image: value.ctx.selected_image.1.to_string().into(),
                destination: value.ctx.selected_destination().into(),
                modifications: value.ctx.customization.modifications(),
                has_customization: value.ctx.has_customization,
            },
            common: value.common,
            ctx: value.ctx,
        }
    }
}

#[derive(Debug)]
pub(crate) struct FlashingCancelState {
    pub(crate) common: BBImagerCommon,
    pub(crate) ctx: FlashingContext,
    pub(crate) state: bb_imager_ui::flash_cancel::State,
}

impl From<FlashingState> for FlashingCancelState {
    fn from(value: FlashingState) -> Self {
        Self {
            state: bb_imager_ui::flash_cancel::State {
                has_customization: value.ctx.has_customization,
            },
            common: value.common,
            ctx: value.ctx,
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
        mut logs: widget::text_editor::Content,
    ) -> Self {
        logs.perform(widget::text_editor::Action::Move(
            widget::text_editor::Motion::DocumentEnd,
        ));
        Self {
            state: bb_imager_ui::flash_fail::State {
                reason: err.into(),
                logs,
                has_customization: state.ctx.has_customization,
            },
            common: state.common,
            ctx: state.ctx,
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
        // Step of the page beneath, and whether its run has Modify. That is not
        // decided before Customize, and Modify cannot be reached from there.
        let (previous, has_customization) = match &page {
            OverlayData::ChooseBoard(_) => (SidebarEntry::Hardware, false),
            OverlayData::ChooseOs(_) => (SidebarEntry::Software, false),
            OverlayData::ChooseDest(_) => (SidebarEntry::Storage, false),
            OverlayData::Customize(_) => (SidebarEntry::Modify, true),
            OverlayData::Review(x) => (SidebarEntry::Review, x.ctx.has_customization),
            OverlayData::Flashing(x) => (SidebarEntry::Write, x.ctx.has_customization),
            OverlayData::FlashingFail(x) => (SidebarEntry::Write, x.ctx.has_customization),
            OverlayData::FlashingSuccess(x) => (SidebarEntry::Write, x.ctx.has_customization),
            OverlayData::FlashingCancel(x) => (SidebarEntry::Write, x.ctx.has_customization),
        };
        let is_flashing = matches!(page, OverlayData::Flashing(_));

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
                previous,
                has_customization,
                is_flashing,
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
