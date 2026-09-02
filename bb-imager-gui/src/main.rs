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
mod focus_scroll;
mod helpers;
mod keyboard;
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

    let icon = iced::window::icon::from_file_data(
        constants::WINDOW_ICON_BYTES,
        Some(image::ImageFormat::Png),
    )
    .ok();
    if icon.is_none() {
        tracing::warn!(
            "Window icon could not be loaded (is git-lfs installed and have you run `git lfs pull`?)"
        );
    }

    let settings = iced::window::Settings {
        min_size: Some(constants::WINDOW_SIZE),
        size: constants::WINDOW_SIZE,
        icon,
        ..Default::default()
    };

    #[cfg(all(target_os = "macos", feature = "notify-rust"))]
    // HACK: mac_notification_sys set application name (not an option in notify-rust)
    let _ = notify_rust::set_application("org.beagleboard.imagingutility");

    iced::application(BBImager::new, message::update, ui::view)
        .title(helpers::app_title)
        .subscription(BBImager::subscription)
        .theme(BBImager::theme)
        .window(settings)
        .font(constants::FONT_NORMAL_BYTES)
        .font(constants::FONT_BOLD_BYTES)
        .default_font(constants::FONT_REGULAR)
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
    Review(state::CustomizeState),
    Flashing(state::FlashingState),
    FlashingCancel(state::FlashingFinishState),
    FlashingFail(state::FlashingFailState),
    FlashingSuccess(state::FlashingFinishState),
    AppInfo(state::OverlayState),
}

impl BBImager {
    fn choose_board(common: BBImagerCommon) -> Self {
        Self::ChooseBoard(state::ChooseBoardState {
            common,
            boards: Box::default(),
            selected_board: None,
            search_text: "".into(),
        })
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
            timezones: widget::combo_box::State::new(chrono_tz::TZ_VARIANTS.to_vec()),
            keymaps: widget::combo_box::State::new(constants::KEYMAP_LAYOUTS.to_vec()),

            img_handle_cache: bb_iced_widgets::cached_icon::Cache::default(),

            scroll_id: widget::Id::unique(),
            search_id: widget::Id::unique(),
            list_selection_id: widget::Id::unique(),
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

    fn theme(&self) -> iced::Theme {
        let high_contrast = self.common().app_config.high_contrast;
        let (name, palette) = constants::theme_palette(high_contrast);
        iced::Theme::custom(name, palette)
    }

    fn scroll_selection(&self) -> Task<BBImagerMessage> {
        crate::focus_scroll::scroll_widget_into_view(self.common().list_selection_id.clone())
    }

    fn focus_search(&self) -> Task<BBImagerMessage> {
        match self {
            Self::ChooseBoard(_) | Self::ChooseOs(_) | Self::ChooseDest(_) => {
                widget::operation::focus(self.common().search_id.clone())
                    .chain(focus_scroll::scroll_focused_into_view())
            }
            _ => Task::none(),
        }
    }

    fn keyboard_tab(&self, shift: bool) -> Task<BBImagerMessage> {
        let focus = if shift {
            widget::operation::focus_previous()
        } else {
            widget::operation::focus_next()
        };

        focus.chain(focus_scroll::scroll_focused_into_view())
    }

    fn keyboard_escape_message(&self) -> Option<BBImagerMessage> {
        match self {
            Self::ChooseBoard(x) if !x.search_text.is_empty() => {
                Some(BBImagerMessage::UpdateSearchText("".into()))
            }
            Self::ChooseOs(x) if !x.search_text.is_empty() => {
                Some(BBImagerMessage::UpdateSearchText("".into()))
            }
            Self::ChooseOs(x) if x.pos.is_some() => Some(BBImagerMessage::GotoOsListParent),
            Self::ChooseDest(x) if !x.search_text.is_empty() => {
                Some(BBImagerMessage::UpdateSearchText("".into()))
            }
            Self::ChooseBoard(_) | Self::Flashing(_) => None,
            Self::AppInfo(_)
            | Self::ChooseOs(_)
            | Self::ChooseDest(_)
            | Self::Customize(_)
            | Self::Review(_) => Some(BBImagerMessage::Back),
            _ => None,
        }
    }

    fn keyboard_enter_message(&self) -> Option<BBImagerMessage> {
        match self {
            Self::ChooseBoard(x) if x.selected_board.is_some() => Some(BBImagerMessage::Next),
            Self::ChooseOs(x) if x.selected_image.is_some() => Some(BBImagerMessage::Next),
            Self::ChooseDest(x) if x.selected_dest.is_some() => Some(BBImagerMessage::Next),
            Self::Customize(x) if x.ctx.customization.validate() => Some(BBImagerMessage::Next),
            Self::Review(_) => Some(BBImagerMessage::FlashStart),
            Self::FlashingFail(_) => Some(BBImagerMessage::Retry),
            Self::FlashingCancel(_) | Self::FlashingSuccess(_) => Some(BBImagerMessage::Restart),
            _ => None,
        }
    }

    fn keyboard_list_message(&self, delta: i32) -> Option<BBImagerMessage> {
        match self {
            Self::ChooseBoard(x) => x.list_select_relative(delta),
            Self::ChooseOs(x) => x.list_select_relative(delta),
            Self::ChooseDest(x) => x.list_select_relative(delta),
            _ => None,
        }
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

        if let BBImager::ChooseBoard(x) = self {
            return x.refresh_board_list();
        }

        Task::none()
    }

    fn subscription(&self) -> Subscription<BBImagerMessage> {
        const INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

        let keyboard = crate::keyboard::subscription();

        let destinations = match self {
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
        };

        Subscription::batch([keyboard, destinations])
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
            ctx,
            cancel_flashing: h,
            progress: bb_flasher::DownloadFlashingStatus::Preparing,
            start_timestamp: None,
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

    fn back(&mut self) -> Task<BBImagerMessage> {
        *self = match std::mem::take(self) {
            Self::ChooseOs(inner) => Self::ChooseBoard(inner.into()),
            Self::ChooseDest(inner) => Self::ChooseOs(inner.into()),
            Self::Customize(inner) => Self::ChooseDest(inner.ctx.choose_dest(inner.common)),
            Self::Review(inner) => {
                if inner.ctx.has_customization {
                    Self::Customize(inner)
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
                    self.refresh_image_icons(board_id),
                    self.scroll_reset(),
                ])
            }
            _ => self.scroll_reset(),
        }
    }

    fn next(&mut self) -> Task<BBImagerMessage> {
        *self = match std::mem::take(self) {
            Self::ChooseBoard(inner) => {
                let selected_board = inner
                    .selected_board
                    .expect("Board should alread have been selected");
                Self::ChooseOs(state::ChooseOsState {
                    common: inner.common,
                    flasher: selected_board.flasher,
                    selected_board,
                    pos: None,
                    selected_image: None,
                    images: Vec::new(),
                    search_text: "".into(),
                })
            }
            Self::ChooseOs(inner) => {
                let selected_image = inner
                    .selected_image
                    .expect("Image should already be selected");

                Self::ChooseDest(state::ChooseDestState {
                    common: inner.common,
                    selected_board: inner.selected_board,
                    selected_image,
                    selected_dest: None,
                    destinations: Box::default(),
                    filter_destination: true,
                    search_text: "".into(),
                })
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

                let page = state::CustomizeState {
                    common: inner.common,
                    ctx: state::FlashingContext {
                        selected_board: inner.selected_board,
                        selected_image: inner.selected_image,
                        selected_dest,
                        customization,
                        has_customization,
                    },
                };

                if has_customization {
                    Self::Customize(page)
                } else {
                    Self::Review(page)
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
                let board_id = inner.selected_board.id;
                Task::batch([
                    inner.resolve_all_remote_sublists(board_id),
                    inner.refresh_image_list(),
                    self.refresh_image_icons(board_id),
                    self.scroll_reset(),
                ])
            }
            Self::Review(inner) => match &inner.ctx.customization {
                // Both variants are backed by the same `sysconf` slot, matching how
                // `FlashingCustomization::new` loads them.
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

                    Task::batch([inner.save_app_config(), self.scroll_reset()])
                }
                _ => self.scroll_reset(),
            },
            _ => self.scroll_reset(),
        }
    }
}
