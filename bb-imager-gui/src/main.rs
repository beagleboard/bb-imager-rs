#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

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
mod img;
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
    fn choose_board(common: BBImagerCommon) -> Self {
        Self::ChooseBoard(state::ChooseBoardState::new(common))
    }

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
            Self::choose_board(common),
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

    fn image_cache_insert(&mut self, k: Arc<url::Url>, v: std::path::PathBuf) {
        self.common_mut().img_handle_cache.insert(k, v)
    }

    fn restart(&mut self) -> Task<BBImagerMessage> {
        *self = match std::mem::take(self) {
            BBImager::ChooseOs(x) => BBImager::choose_board(x.common),
            BBImager::ChooseDest(x) => BBImager::choose_board(x.common),
            BBImager::Customize(x) => BBImager::choose_board(x.common),
            BBImager::Review(x) => BBImager::choose_board(x.common),
            BBImager::Flashing(x) => BBImager::choose_board(x.common),
            BBImager::FlashingCancel(x) => BBImager::choose_board(x.common),
            BBImager::FlashingSuccess(x) => BBImager::choose_board(x.common),
            BBImager::FlashingFail(x) => BBImager::choose_board(x.common),
            BBImager::Dummy | BBImager::AppInfo(_) | BBImager::ChooseBoard(_) => {
                panic!("Unexpected screen")
            }
        };

        if let BBImager::ChooseBoard(x) = self {
            return x.refresh_board_list();
        }

        Task::none()
    }

    fn subscription(&self) -> Subscription<BBImagerMessage> {
        const INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

        match self {
            Self::ChooseDest(x) => Subscription::run_with(
                (
                    x.selected_image.1.flasher(),
                    x.filter_destination,
                    Arc::<str>::from(x.search_text.to_lowercase()),
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
        let img = ctx.selected_image.1.clone();
        let dst = ctx.selected_dest.clone();
        let bootfs = ctx.selected_board.bootfs.as_ref().map(|x| {
            img::RemoteItem::new(
                Box::new(x.url.clone()),
                x.image_download_sha256,
                x.image_download_size,
                common.downloader.clone(),
            )
        });

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
                helpers::flash(img, customization, dst, bootfs, tx, cancel_child).await
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
            // Built before `ctx` is moved in below.
            state: Box::new(bb_imager_ui::flashing::State {
                board: (&ctx.selected_board).into(),
                progress: Default::default(),
                start_timestamp: None,
            }),
            ctx,
        });

        t
    }

    fn scroll_reset(&self) -> Task<BBImagerMessage> {
        widget::operation::snap_to(
            self.common().scroll_id.clone(),
            widget::operation::RelativeOffset::START,
        )
    }

    fn back(&mut self) -> Task<BBImagerMessage> {
        *self = match std::mem::take(self) {
            Self::ChooseOs(inner) => Self::ChooseBoard(inner.into()),
            Self::ChooseDest(inner) => Self::ChooseOs(inner.into()),
            Self::Customize(inner) => Self::ChooseDest(inner.ctx.choose_dest(inner.common)),
            Self::Review(inner) => {
                if inner.ctx.has_customization {
                    Self::Customize(inner.into())
                } else {
                    Self::ChooseDest(inner.ctx.choose_dest(inner.common))
                }
            }
            Self::AppInfo(inner) => inner.page.into(),
            Self::Dummy
            | Self::FlashingSuccess(_)
            | Self::FlashingFail(_)
            | Self::FlashingCancel(_)
            | Self::Flashing(_)
            | Self::ChooseBoard(_) => panic!("Unexpected message"),
        };

        match self {
            BBImager::ChooseBoard(inner) => {
                Task::batch([inner.refresh_board_list(), self.scroll_reset()])
            }
            BBImager::ChooseOs(inner) => {
                let board_id = inner.selected_board.id;
                Task::batch([
                    inner.refresh_image_list(),
                    self.common().refresh_image_icons(board_id),
                    self.scroll_reset(),
                ])
            }
            _ => self.scroll_reset(),
        }
    }

    fn next(&mut self) -> Task<BBImagerMessage> {
        let (state, task) = match std::mem::take(self) {
            Self::ChooseBoard(inner) => {
                let selected_board = inner
                    .selected_board
                    .expect("Board should alread have been selected");
                let board_id = selected_board.id;

                let temp = state::ChooseOsState {
                    common: inner.common,
                    flasher: selected_board.flasher,
                    selected_board,
                    pos: None,
                    selected_image: None,
                    images: Vec::new(),
                    search_text: "".into(),
                };

                let tasks = Task::batch([
                    temp.resolve_all_remote_sublists(board_id),
                    temp.refresh_image_list(),
                    temp.common.refresh_image_icons(board_id),
                ]);

                (Self::ChooseOs(temp), tasks)
            }
            Self::ChooseOs(inner) => {
                let selected_image = inner
                    .selected_image
                    .expect("Image should already be selected");

                (
                    Self::ChooseDest(state::ChooseDestState {
                        common: inner.common,
                        selected_board: inner.selected_board,
                        selected_image,
                        selected_dest: None,
                        destinations: Box::default(),
                        filter_destination: true,
                        search_text: "".into(),
                    }),
                    Task::none(),
                )
            }
            Self::ChooseDest(inner) => {
                let selected_dest = inner
                    .selected_dest
                    .expect("Destination should already be selcted");

                let flasher = inner.selected_image.1.flasher();

                // A flasher with nothing to configure skips the Customize page.
                let (customization, has_customization) =
                    match helpers::no_customization(flasher, &inner.selected_image.1) {
                        Some(c) => (c, false),
                        None => (
                            helpers::FlashingCustomization::new(
                                flasher,
                                &inner.selected_image.1,
                                &inner.common.app_config,
                            ),
                            true,
                        ),
                    };

                let ctx = state::FlashingContext {
                    selected_board: inner.selected_board,
                    selected_image: inner.selected_image,
                    selected_dest,
                    customization,
                    has_customization,
                };

                let temp = if has_customization {
                    Self::Customize(state::CustomizeState::new(inner.common, ctx))
                } else {
                    Self::Review(state::ReviewState::new(inner.common, ctx))
                };

                (temp, Task::none())
            }
            Self::Customize(mut inner) => {
                // The page edits its own state, so the context only catches up
                // here, on the way to Review.
                inner.ctx.customization = (&inner.state.customization).into();

                let temp = match &inner.ctx.customization {
                    helpers::FlashingCustomization::LinuxSdSysconfig(c)
                    | helpers::FlashingCustomization::LinuxSdCloudInit(c) => {
                        let mut temp = inner
                            .common
                            .app_config
                            .sd_customization
                            .clone()
                            .unwrap_or_default();
                        temp.update_sysconfig(c.clone());
                        inner.common.app_config.update_sd_customization(temp);

                        inner.save_app_config()
                    }
                    _ => Task::none(),
                };

                (
                    Self::Review(state::ReviewState::new(inner.common, inner.ctx)),
                    temp,
                )
            }
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

        *self = state;

        Task::batch([task, self.scroll_reset()])
    }
}
