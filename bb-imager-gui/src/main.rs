#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use constants::PACKAGE_QUALIFIER;
use iced::{Subscription, Task, futures::SinkExt, widget};
use message::BBImagerMessage;
use tokio::time::interval;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{helpers::blocking_future, state::BBImagerCommon};

mod constants;
mod db;
mod helpers;
mod message;
mod persistance;
mod state;
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
                .with_writer(std::fs::File::create(&log_file_p).unwrap()),
        )
        .try_init()
        .expect("Failed to register tracing_subscriber");

    tracing::info!("Resolved GUI keymap: {:?}", helpers::system_keymap());

    // Force using the low power gpu since this is not a GPU intensive application
    unsafe { std::env::set_var("WGPU_POWER_PREF", "low") };

    #[cfg(target_os = "macos")]
    // HACK: mac_notification_sys set application name (not an option in notify-rust)
    let _ = notify_rust::set_application("org.beagleboard.imagingutility");

    let app = iced::application(BBImager::new, message::update, ui::view);
    bb_imager_ui::application(app)
        .title(helpers::app_title)
        .subscription(BBImager::subscription)
        .run()
}

#[derive(Default)]
enum BBImager {
    // Dummy state to allow clone-free move among variants. Should never be exposed in view.
    #[default]
    Dummy,
    ChooseBoard(state::ChooseBoardState),
    ChooseOs(state::ChooseOsState),
    ChooseDest(state::ChooseDestState),
    Customize(state::CustomizeState),
    Review(state::ReviewState),
    Flashing(state::FlashingState),
    FlashingCancel(state::FlashingCancelState),
    FlashingFail(state::FlashingFailState),
    FlashingSuccess(state::FlashingSuccessState),
    AppInfo(state::OverlayState),
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

        let db = crate::db::Db::new().unwrap();

        let common = BBImagerCommon {
            app_config,
            downloader: downloader.clone(),
            img_handle_cache: bb_iced_widgets::cached_icon::Cache::default(),
            scroll_id: widget::Id::unique(),
            db: db.clone(),
        };

        let db_task = Task::future(blocking_future(move || {
            db.init().expect("Failed to initialize db");
            BBImagerMessage::DbInitSuccess
        }));
        let updater_task = common.updater_task();

        (
            BBImager::ChooseBoard(state::ChooseBoardState::new(common)),
            Task::batch([db_task, updater_task]),
        )
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

    fn subscription(&self) -> Subscription<BBImagerMessage> {
        const INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

        match self {
            Self::ChooseDest(x) => Subscription::run_with(
                (
                    x.selected_image.flasher(),
                    x.inner.filter_destination,
                    x.inner.search.to_lowercase(),
                ),
                |(flasher, filter, search_text)| {
                    let mut interval = interval(INTERVAL);
                    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

                    iced::futures::stream::unfold(
                        (*flasher, *filter, search_text.clone(), interval),
                        async move |(flasher, filter, search_text, mut interval)| {
                            interval.tick().await;
                            let search = search_text.clone();
                            let dest = blocking_future(move || {
                                helpers::destinations(flasher, filter, search)
                            })
                            .await;

                            let msg = BBImagerMessage::Destinations(dest);
                            Some((msg, (flasher, filter, search_text, interval)))
                        },
                    )
                },
            ),
            _ => Subscription::none(),
        }
    }

    fn start_flashing(&mut self) -> Task<BBImagerMessage> {
        // Retrying from the failure page re-runs with the same choices.
        let (common, ctx) = match std::mem::take(self) {
            Self::Review(inner) => (inner.common, inner.ctx),
            Self::FlashingFail(inner) => (inner.common, inner.ctx),
            _ => panic!("Unexpected page"),
        };

        let customization = ctx.customization.clone();
        let img = ctx.selected_image.clone();
        let dst = ctx.selected_dest.clone();

        tracing::info!("Starting Flashing Process");
        tracing::info!("Selected Board: {:#?}", ctx.selected_board);
        tracing::info!("Selected Image: {:#?}", img);
        tracing::info!("Selected Destination: {:#?}", dst);
        tracing::info!("Selected Customization: {:#?}", customization);

        let cancel = bb_helper::cancel::CancellationToken::default();

        let s = iced::stream::channel(2, async move |mut chan| {
            let (tx, rx) = std::sync::mpsc::sync_channel(2);

            let cancel_child = cancel.clone();
            let flash_task = tokio::spawn(async move {
                helpers::flash(img, customization, dst, tx, cancel_child).await
            });
            let mut chan_clone = chan.clone();
            let progress_task = tokio::task::spawn_blocking(move || {
                while let Ok(progress) = rx.recv() {
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

        *self = Self::Flashing(state::FlashingState {
            common,
            cancel_flashing: h,
            inner: bb_imager_ui::flashing::State {
                has_customization: ctx.has_customization,
                ..Default::default()
            },
            ctx,
        });

        t
    }

    fn refresh_image_icons(&self, board_id: i64) -> Task<BBImagerMessage> {
        let db = self.common().db.clone();
        Task::perform(
            blocking_future(move || db.os_image_icons_by_board_id(board_id).unwrap()),
            BBImagerMessage::FilterResolveImages,
        )
    }

    fn scroll_reset(&self) -> Task<BBImagerMessage> {
        widget::operation::snap_to(
            self.common().scroll_id.clone(),
            widget::operation::RelativeOffset::START,
        )
    }
}
