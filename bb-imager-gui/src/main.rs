#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time::Duration;

use constants::PACKAGE_QUALIFIER;
use iced::{Subscription, Task, futures::SinkExt, widget};
use message::BBImagerMessage;
use tokio_stream::StreamExt as _;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::state::BBImagerCommon;

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
                .with_writer(std::fs::File::create(helpers::log_file_path()).unwrap()),
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
    const fn choose_board(common: BBImagerCommon) -> Self {
        Self::ChooseBoard(state::ChooseBoardState {
            common,
            boards: Vec::new(),
            selected_board: None,
            search_text: String::new(),
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
            timezones: widget::combo_box::State::new(
                constants::TIMEZONES.iter().map(|x| x.to_string()).collect(),
            ),
            keymaps: widget::combo_box::State::new(
                constants::KEYMAP_LAYOUTS
                    .iter()
                    .map(|x| x.to_string())
                    .collect(),
            ),

            img_handle_cache: helpers::ImageHandleCache::default(),

            scroll_id: widget::Id::unique(),
            db: db.clone(),
        };

        let db_task = Task::future(async move {
            db.init().await.expect("Failed to initialize db");
            BBImagerMessage::DbInitSuccess
        });
        let updater_task = common.updater_task();

        (
            Self::choose_board(common),
            Task::batch([db_task, updater_task]),
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

    fn image_cache_extend(&mut self, iter: Vec<(url::Url, std::path::PathBuf)>) {
        self.common_mut().img_handle_cache.extend(iter)
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

        unreachable!()
    }

    fn subscription(&self) -> Subscription<BBImagerMessage> {
        match self {
            Self::ChooseDest(x) => Subscription::run_with(
                (x.selected_image.1.flasher(), x.filter_destination),
                |(flasher, filter)| {
                    iced::futures::stream::unfold(
                        (*flasher, *filter),
                        async move |(flasher, filter)| {
                            let mut dest = helpers::destinations(flasher, filter).await;

                            dest.sort_by_key(|x| x.to_string());

                            let msg = BBImagerMessage::Destinations(dest);
                            Some((msg, (flasher, filter)))
                        },
                    )
                    .throttle(Duration::from_secs(1))
                },
            ),
            _ => Subscription::none(),
        }
    }

    fn start_flashing(&mut self) -> Task<BBImagerMessage> {
        let state = match std::mem::take(self) {
            Self::Review(inner) => inner,
            _ => panic!("Unexpected page"),
        };

        let is_download = state.is_download();
        let customization = state.customization;
        let img = state.selected_image.1.clone();
        let dst = state.selected_dest;

        tracing::info!("Starting Flashing Process");
        tracing::info!("Selected Board: {:#?}", state.selected_board);
        tracing::info!("Selected Image: {:#?}", img);
        tracing::info!("Selected Destination: {:#?}", dst);
        tracing::info!("Selected Customization: {:#?}", customization);

        let cancel = tokio_util::sync::CancellationToken::new();

        let s = iced::stream::channel(20, async move |mut chan| {
            let (tx, mut rx) = tokio::sync::mpsc::channel(19);

            let cancel_child = cancel.child_token();
            let flash_task = tokio::spawn(async move {
                helpers::flash(img, customization, dst, tx, cancel_child).await
            });
            let mut chan_clone = chan.clone();
            let progress_task = tokio::spawn(async move {
                while let Some(progress) = rx.recv().await {
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
            is_download,
            common: state.common,
            selected_board: state.selected_board,
            cancel_flashing: h,
            progress: bb_flasher::DownloadFlashingStatus::Preparing,
            start_timestamp: None,
        });

        t
    }

    fn resolve_remote_sublists(&self, board_id: i64, pos: Option<i64>) -> Task<BBImagerMessage> {
        let db = self.common().db.clone();
        let downloader = self.common().downloader.clone();

        Task::future(async move { db.os_remote_sublists(board_id, pos).await.unwrap() }).then(
            move |items| {
                let dl = downloader.clone();
                let temp = items.into_iter().map(move |(id, url)| {
                    let url_clone = url::Url::from(url.clone());
                    let dl = dl.clone();
                    Task::perform(
                        async move { dl.download_json_no_cache(url_clone).await },
                        move |x| match x {
                            Ok(json) => BBImagerMessage::ResolveRemoteSubitemItem {
                                item: json,
                                target: id,
                            },
                            Err(e) => {
                                tracing::error!("Failed to get remote item {}: {e}", url.as_str());
                                BBImagerMessage::Null
                            }
                        },
                    )
                });

                Task::batch(temp)
            },
        )
    }

    fn refresh_image_icons(&self, board_id: i64) -> Task<BBImagerMessage> {
        let db = self.common().db.clone();
        Task::perform(
            async move {
                db.os_image_icons_by_board_id(board_id)
                    .await
                    .unwrap()
                    .into_iter()
                    .map(Into::into)
                    .collect()
            },
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
            Self::Customize(inner) => Self::ChooseDest(inner.into()),
            Self::Review(inner) => {
                if helpers::no_customization(
                    inner.selected_image.1.flasher(),
                    &inner.selected_image.1,
                    &inner.selected_dest,
                )
                .is_none()
                {
                    Self::Customize(inner)
                } else {
                    Self::ChooseDest(inner.into())
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
                    search_text: String::new(),
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
                    destinations: Vec::new(),
                    filter_destination: true,
                })
            }
            Self::ChooseDest(inner) => {
                let selected_dest = inner
                    .selected_dest
                    .expect("Destination should already be selcted");

                if let Some(customization) = helpers::no_customization(
                    inner.selected_image.1.flasher(),
                    &inner.selected_image.1,
                    &selected_dest,
                ) {
                    Self::Review(state::CustomizeState {
                        common: inner.common,
                        selected_board: inner.selected_board,
                        selected_image: inner.selected_image,
                        selected_dest,
                        customization,
                    })
                } else {
                    let temp = helpers::FlashingCustomization::new(
                        inner.selected_image.1.flasher(),
                        &inner.selected_image.1,
                        &inner.common.app_config,
                    );

                    Self::Customize(state::CustomizeState {
                        common: inner.common,
                        selected_board: inner.selected_board,
                        selected_image: inner.selected_image,
                        selected_dest,
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
                let board_id = inner.selected_board.id;
                Task::batch([
                    inner.refresh_image_list(),
                    self.resolve_remote_sublists(board_id, None),
                    self.refresh_image_icons(board_id),
                    self.scroll_reset(),
                ])
            }
            Self::Review(inner) => match &inner.customization {
                helpers::FlashingCustomization::LinuxSdSysconfig(c) => {
                    let mut temp = inner
                        .app_config()
                        .sd_customization
                        .clone()
                        .unwrap_or_default();
                    temp.update_sysconfig(c.clone());
                    inner.common.app_config.update_sd_customization(temp);

                    Task::batch([inner.save_app_config(), self.scroll_reset()])
                }
                helpers::FlashingCustomization::Bcf(c) => {
                    inner.common.app_config.update_bcf_customization(c.clone());
                    Task::batch([inner.save_app_config(), self.scroll_reset()])
                }
                helpers::FlashingCustomization::Zepto(c) => {
                    inner
                        .common
                        .app_config
                        .update_zepto_customization(c.clone());
                    Task::batch([inner.save_app_config(), self.scroll_reset()])
                }
                _ => self.scroll_reset(),
            },
            _ => self.scroll_reset(),
        }
    }
}
