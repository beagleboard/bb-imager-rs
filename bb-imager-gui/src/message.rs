//! Global GUI Messages

use bb_imager_ui::{Message, customization::Customization};
use iced::Task;

use crate::{
    BBImager,
    helpers::{self, blocking_future},
    state::{OverlayData, OverlayState},
};

#[derive(Debug, Clone)]
pub(crate) enum BBImagerMessage {
    UiState(Message<helpers::Destination>),

    /// Messages to ignore
    Null,

    /// Config related options
    ExtendConfig((i64, bb_config::Config)),
    ResolveRemoteSubitemItem {
        item: Box<[bb_config::config::OsListItem]>,
        target: i64,
    },

    /// A new version of application is available
    UpdateAvailable(semver::Version),

    /// Select a board by index. Can only be used in Board selection page.
    UpdateBoardList(Box<[bb_imager_ui::board_selection::Board]>),
    SelectBoard(crate::db::Board),

    /// ChooseOs Page
    UpdateOsList((Vec<bb_imager_ui::img_selection::ImageItem>, Option<i64>)),

    /// An image that needs no lookup: a local file or the SD format action.
    SelectImage(helpers::BoardImage),
    SelectRemoteOs((crate::db::OsImage, bb_config::config::Flasher)),

    /// Choose Destination page
    SelectFileDest(std::sync::Arc<str>),

    // Flashing Page
    FlashProgress(bb_flasher::DownloadFlashingStatus),
    FlashSuccess,
    FlashFail(String),

    // Download images which have not already been downloaded
    FilterResolveImages(Vec<std::sync::Arc<url::Url>>),

    /// Update destinations
    Destinations(Box<[helpers::Destination]>),

    /// DB Ops
    DbInitSuccess,
}

impl From<Message<helpers::Destination>> for BBImagerMessage {
    fn from(value: Message<helpers::Destination>) -> Self {
        BBImagerMessage::UiState(value)
    }
}

pub(crate) fn update(state: &mut BBImager, message: BBImagerMessage) -> Task<BBImagerMessage> {
    match message {
        BBImagerMessage::UiState(Message::UpdateSearchText(x)) => match state {
            BBImager::ChooseBoard(y) => return y.update_search(x),
            BBImager::ChooseOs(y) => return y.update_search(x),
            BBImager::ChooseDest(y) => y.update_search(x),
            _ => unreachable!(),
        },
        BBImagerMessage::UiState(Message::SelectBoardById(id)) => {
            let db = state.common().db.clone();
            return Task::perform(
                blocking_future(move || db.board_by_id(id).expect("Incorrect board id")),
                BBImagerMessage::SelectBoard,
            );
        }
        BBImagerMessage::UiState(
            Message::GotoDevicePage
            | Message::GotoSoftwarePage
            | Message::GotoDestinationPage
            | Message::GotoCustomizationPage
            | Message::GotoReviewPage
            | Message::GotoFlashingPage,
        ) if matches!(state, BBImager::AppInfo(_)) => {
            let BBImager::AppInfo(overlay) = std::mem::take(state) else {
                unreachable!()
            };
            *state = overlay.page.into();
            return state.scroll_reset();
        }
        BBImagerMessage::UiState(Message::GotoDevicePage) => {
            let inner = match std::mem::take(state) {
                BBImager::ChooseBoard(x) => x,
                BBImager::ChooseOs(x) => crate::state::ChooseBoardState::new(x.common),
                BBImager::ChooseDest(x) => crate::state::ChooseBoardState::new(x.common),
                BBImager::Customize(x) => crate::state::ChooseBoardState::new(x.common),
                BBImager::Review(x) => crate::state::ChooseBoardState::new(x.common),
                BBImager::FlashingCancel(x) => crate::state::ChooseBoardState::new(x.common),
                BBImager::FlashingFail(x) => crate::state::ChooseBoardState::new(x.common),
                BBImager::FlashingSuccess(x) => crate::state::ChooseBoardState::new(x.common),
                _ => unreachable!(),
            };

            let task = inner.refresh_board_list();
            *state = BBImager::ChooseBoard(inner);

            return Task::batch([task, state.scroll_reset()]);
        }
        BBImagerMessage::UiState(Message::GotoSoftwarePage) => {
            let inner: crate::state::ChooseOsState = match std::mem::take(state) {
                BBImager::ChooseOs(x) => x,
                BBImager::ChooseDest(x) => x.into(),
                BBImager::Customize(x) => x.into(),
                BBImager::Review(x) => x.into(),
                BBImager::FlashingCancel(x) => x.ctx.choose_os(x.common),
                BBImager::FlashingFail(x) => x.ctx.choose_os(x.common),
                BBImager::FlashingSuccess(x) => x.ctx.choose_os(x.common),
                _ => unreachable!(),
            };

            let task = inner.refresh_image_list();
            *state = BBImager::ChooseOs(inner);

            return Task::batch([task, state.scroll_reset()]);
        }
        BBImagerMessage::UiState(Message::GotoDestinationPage) => {
            let inner = match std::mem::take(state) {
                BBImager::ChooseDest(x) => x,
                BBImager::Customize(x) => x.into(),
                BBImager::Review(x) => x.into(),
                BBImager::FlashingCancel(x) => x.ctx.choose_dest(x.common),
                BBImager::FlashingFail(x) => x.ctx.choose_dest(x.common),
                BBImager::FlashingSuccess(x) => x.ctx.choose_dest(x.common),
                _ => unreachable!(),
            };

            *state = BBImager::ChooseDest(inner);

            return state.scroll_reset();
        }
        BBImagerMessage::UiState(Message::GotoCustomizationPage) => {
            let inner = match std::mem::take(state) {
                BBImager::Customize(x) => x,
                BBImager::Review(x) => x.into(),
                BBImager::FlashingCancel(x) => x.ctx.customize(x.common),
                BBImager::FlashingFail(x) => x.ctx.customize(x.common),
                BBImager::FlashingSuccess(x) => x.ctx.customize(x.common),
                _ => unreachable!(),
            };

            *state = BBImager::Customize(inner);

            return state.scroll_reset();
        }
        BBImagerMessage::UiState(Message::GotoReviewPage) => {
            let inner = match std::mem::take(state) {
                BBImager::Review(x) => x,
                BBImager::FlashingCancel(x) => x.ctx.review(x.common),
                BBImager::FlashingFail(x) => x.ctx.review(x.common),
                BBImager::FlashingSuccess(x) => x.ctx.review(x.common),
                _ => unreachable!(),
            };

            *state = BBImager::Review(inner);

            return state.scroll_reset();
        }
        BBImagerMessage::UiState(Message::SelectOs(
            bb_imager_ui::img_selection::ImageId::Local(flasher),
        )) => {
            let extensions = helpers::file_filter(flasher);

            return Task::perform(
                async move {
                    rfd::AsyncFileDialog::new()
                        .add_filter("image", extensions)
                        .pick_file()
                        .await
                        .map(|x| x.inner().to_path_buf())
                },
                move |x| match x {
                    Some(y) => BBImagerMessage::SelectImage(helpers::BoardImage::local(y, flasher)),
                    None => BBImagerMessage::Null,
                },
            );
        }
        BBImagerMessage::UiState(Message::SelectOs(
            bb_imager_ui::img_selection::ImageId::Format,
        )) => {
            return Task::done(BBImagerMessage::SelectImage(helpers::BoardImage::SdFormat));
        }
        BBImagerMessage::UiState(Message::SelectOs(
            bb_imager_ui::img_selection::ImageId::OsSublist((id, flasher)),
        )) if let BBImager::ChooseOs(inner) = state => {
            let board_id = inner.selected_board.id;
            return Task::batch([
                inner.resolve_remote_sublists(board_id, Some(id)),
                inner.update_pos(Some(id), flasher),
            ]);
        }

        BBImagerMessage::UiState(Message::SelectOs(
            bb_imager_ui::img_selection::ImageId::OsImage(id),
        )) if let BBImager::ChooseOs(inner) = state => {
            let db = inner.common.db.clone();
            let flasher = inner.flasher;
            return Task::perform(
                blocking_future(move || db.os_image_by_id(id)),
                move |x| match x {
                    Ok(i) => BBImagerMessage::SelectRemoteOs((i, flasher)),
                    Err(e) => {
                        tracing::error!("Failed to get os image {e}");
                        BBImagerMessage::Null
                    }
                },
            );
        }
        BBImagerMessage::UiState(Message::GotoOsListParent)
            if let BBImager::ChooseOs(inner) = state =>
        {
            let db = inner.common.db.clone();
            let curpos = inner.inner.pos.unwrap();
            let board_id = inner.selected_board.id;

            return Task::perform(
                blocking_future(move || {
                    let id = db.os_sublist_parent(curpos).unwrap();
                    let imgs = db.os_image_items(board_id, id).unwrap();
                    (imgs, id)
                }),
                BBImagerMessage::UpdateOsList,
            );
        }
        BBImagerMessage::UiState(Message::DestinationFilter(x))
            if let BBImager::ChooseDest(inner) = state =>
        {
            inner.inner.filter_destination = x;
        }
        BBImagerMessage::UiState(Message::UpdateCustomizaton(x))
            if let BBImager::Customize(inner) = state =>
        {
            inner.inner.customization = x;
        }
        BBImagerMessage::UiState(Message::OpenUrl(x)) => {
            return Task::future(async move {
                let res = webbrowser::open(x.as_str());
                tracing::debug!("Open Url Resp {res:?}");
                BBImagerMessage::Null
            });
        }
        BBImagerMessage::SelectImage(img) => match std::mem::take(state) {
            BBImager::ChooseOs(page) => {
                *state = BBImager::ChooseDest(crate::state::ChooseDestState::new(
                    page.common,
                    page.selected_board,
                    img,
                ));

                return state.scroll_reset();
            }
            _ => unimplemented!(),
        },
        BBImagerMessage::UiState(Message::Next) => match std::mem::take(state) {
            BBImager::Customize(mut inner) => {
                match &inner.inner.customization {
                    Customization::SysConfig(x)
                    | Customization::SelectableSd(
                        bb_imager_ui::customization::SelectableSd::SysConfig(x),
                    ) => {
                        let mut temp = inner.common.app_config.sd_customization.clone();
                        temp.update_sysconfig(x.into());

                        inner.common.app_config.update_sd_customization(temp);
                    }
                    Customization::CloudInit(x)
                    | Customization::SelectableSd(
                        bb_imager_ui::customization::SelectableSd::CloudInit(x),
                    ) => {
                        let mut temp = inner.common.app_config.sd_customization.clone();
                        temp.update_sysconfig(x.into());

                        inner.common.app_config.update_sd_customization(temp);
                    }
                    Customization::SelectableSd(
                        bb_imager_ui::customization::SelectableSd::None,
                    ) => {}
                };

                let task = inner.save_app_config();
                let customization: helpers::FlashingCustomization =
                    (&inner.inner.customization).into();

                let ctx = crate::state::FlashingContext {
                    selected_board: inner.selected_board,
                    selected_image: inner.selected_image,
                    selected_dest: inner.selected_dest,
                    customization,
                    has_customization: true,
                };

                *state = BBImager::Review(ctx.review(inner.common));

                return Task::batch([task, state.scroll_reset()]);
            }
            _ => unimplemented!(),
        },
        BBImagerMessage::SelectBoard(board) => {
            let flasher = board.flasher;
            let board_id = board.id;

            match std::mem::take(state) {
                BBImager::ChooseBoard(page) => {
                    let inner = crate::state::ChooseOsState {
                        common: page.common,
                        selected_board: board,
                        flasher,
                        inner: Default::default(),
                    };

                    let tasks = [
                        inner.resolve_all_remote_sublists(board_id),
                        inner.refresh_image_list(),
                        inner.refresh_image_icons(board_id),
                    ];

                    *state = BBImager::ChooseOs(inner);

                    return Task::batch([state.scroll_reset()].into_iter().chain(tasks));
                }
                _ => unimplemented!(),
            }
        }
        BBImagerMessage::SelectRemoteOs((image, flasher)) => match std::mem::take(state) {
            BBImager::ChooseOs(page) => {
                let img =
                    helpers::BoardImage::remote(image, flasher, page.common.downloader.clone());
                *state = BBImager::ChooseDest(crate::state::ChooseDestState::new(
                    page.common,
                    page.selected_board,
                    img,
                ));

                return state.scroll_reset();
            }
            _ => unimplemented!(),
        },
        BBImagerMessage::UiState(Message::SelectDestination(x)) => {
            match std::mem::take(state) {
                BBImager::ChooseDest(page) => {
                    let flasher = page.selected_image.flasher();
                    let img = page.selected_image;

                    // Nothing to customize for this image, so skip that page entirely.
                    if let Some(customization) = helpers::no_customization(flasher, &img) {
                        let ctx = crate::state::FlashingContext {
                            selected_board: page.selected_board,
                            selected_image: img,
                            selected_dest: x,
                            customization,
                            has_customization: false,
                        };

                        *state = BBImager::Review(ctx.review(page.common));
                    } else {
                        let customization = match flasher {
                            bb_config::config::Flasher::SdCard
                                if img.init_format() == bb_config::config::InitFormat::Sysconf =>
                            {
                                Customization::SysConfig((&page.common.app_config).into())
                            }
                            bb_config::config::Flasher::SdCard
                                if img.init_format()
                                    == bb_config::config::InitFormat::CloudInit =>
                            {
                                Customization::CloudInit((&page.common.app_config).into())
                            }
                            bb_config::config::Flasher::SdCard => Customization::SelectableSd(
                                bb_imager_ui::customization::SelectableSd::None,
                            ),
                            _ => todo!(),
                        };

                        *state = BBImager::Customize(crate::state::CustomizeState {
                            common: page.common,
                            selected_board: page.selected_board,
                            selected_image: img,
                            selected_dest: x,
                            inner: bb_imager_ui::customization::State {
                                customization,
                                default_username: helpers::default_user(),
                                default_timezone: helpers::system_timezone(),
                                default_keymap: helpers::system_keymap(),
                                timezones: iced::widget::combo_box::State::new(
                                    chrono_tz::TZ_VARIANTS.to_vec(),
                                ),
                                keymaps: iced::widget::combo_box::State::new(
                                    crate::constants::KEYMAP_LAYOUTS.to_vec(),
                                ),
                            },
                        })
                    }
                }
                _ => unimplemented!(),
            }

            return state.scroll_reset();
        }
        BBImagerMessage::UiState(Message::Reset) if let BBImager::Customize(x) = state => {
            let customization = match x.inner.customization {
                Customization::SelectableSd(_) => Customization::SelectableSd(Default::default()),
                Customization::SysConfig(_) => Customization::SysConfig(Default::default()),
                Customization::CloudInit(_) => Customization::CloudInit(Default::default()),
            };
            x.inner.customization = customization;
        }
        BBImagerMessage::UiState(Message::SelectInitFormat(x))
            if let BBImager::Customize(inner) = state =>
        {
            let temp = match x {
                bb_config::config::InitFormat::None => {
                    Customization::SelectableSd(bb_imager_ui::customization::SelectableSd::None)
                }
                bb_config::config::InitFormat::Sysconf => {
                    Customization::SysConfig((&inner.common.app_config).into())
                }
                bb_config::config::InitFormat::CloudInit => {
                    Customization::CloudInit((&inner.common.app_config).into())
                }
                _ => unimplemented!(),
            };

            inner.inner.customization = temp;
        }
        BBImagerMessage::UpdateBoardList(boards) => {
            // Update board list only if still on that page
            match state {
                BBImager::ChooseBoard(x) => {
                    x.inner.boards = boards;
                }
                BBImager::AppInfo(overlay_state) => {
                    if let OverlayData::ChooseBoard(x) = &mut overlay_state.page {
                        x.inner.boards = boards;
                    }
                }
                _ => {}
            }
        }
        BBImagerMessage::UpdateOsList((imgs, pos)) => {
            match state {
                BBImager::ChooseOs(inner) => inner.update_images(imgs, pos),
                BBImager::AppInfo(overlay_state) => {
                    if let OverlayData::ChooseOs(inner) = &mut overlay_state.page {
                        inner.update_images(imgs, pos)
                    }
                }
                _ => {}
            };
        }
        BBImagerMessage::FilterResolveImages(x) => {
            let common = state.common_mut();
            let iter = x.into_iter().filter(|x| {
                if common.img_handle_cache.contains(x) {
                    false
                } else {
                    common.img_handle_cache.mark_fetching(x.clone());
                    true
                }
            });
            return helpers::fetch_images(&common.downloader, iter);
        }
        BBImagerMessage::ExtendConfig((u, c)) => {
            tracing::debug!("Update Config: {:#?}", c);

            let db = state.common().db.clone();
            let db_task = Task::perform(blocking_future(move || db.add_config(c, Some(u))), |x| {
                if let Err(e) = x {
                    tracing::error!("Failed to merge config {e}");
                }
                BBImagerMessage::Null
            });

            let tail_tasks = match state {
                // If we are in ChooseBoard page, update the board list
                BBImager::ChooseBoard(inner) => Task::batch([
                    inner.common.fetch_board_images(),
                    inner.refresh_board_list(),
                ]),
                BBImager::ChooseOs(inner) => {
                    let board_id = inner.selected_board.id;
                    let db = inner.common.db.clone();
                    let downloader = inner.common.downloader.clone();

                    let remote_items_fetch = Task::future(blocking_future(move || {
                        db.os_remote_sublists_by_remote_config(board_id, u).unwrap()
                    }))
                    .then(move |items| {
                        let dl = downloader.clone();
                        helpers::fetch_remote_subitems(items, dl)
                    });

                    Task::batch([inner.common.fetch_board_images(), remote_items_fetch])
                }
                _ => state.common().fetch_board_images(),
            };

            // We want fetch board images to run after the config has been added
            return db_task.chain(tail_tasks);
        }
        BBImagerMessage::ResolveRemoteSubitemItem { item, target } => {
            let db = state.common().db.clone();
            let tail = match &state {
                BBImager::ChooseOs(inner) => Task::batch([
                    // Fetch all children remote subitems.
                    inner.resolve_remote_sublists(inner.selected_board.id, Some(target)),
                    inner.refresh_image_list(),
                    state.refresh_image_icons(inner.selected_board.id),
                ]),
                _ => Task::none(),
            };

            return Task::future(blocking_future(move || {
                db.os_remote_sublist_resolve(target, &item).unwrap();
                BBImagerMessage::Null
            }))
            .chain(tail);
        }
        BBImagerMessage::UpdateAvailable(x) => {
            return show_notification(format!("A new version of application is available {}", x));
        }
        BBImagerMessage::Destinations(x) if let BBImager::ChooseDest(inner) = state => {
            inner.inner.destinations = x;
        }
        BBImagerMessage::SelectFileDest(x) => {
            return Task::perform(
                async move {
                    rfd::AsyncFileDialog::new()
                        .set_file_name(x.as_ref())
                        .save_file()
                        .await
                        .map(|x| x.inner().to_path_buf())
                },
                move |x| match x {
                    Some(y) => BBImagerMessage::UiState(Message::SelectDestination(
                        helpers::Destination::LocalFile(y),
                    )),
                    None => BBImagerMessage::Null,
                },
            );
        }
        BBImagerMessage::UiState(Message::FlashCancel) => {
            let mut msg = "Flashing cancelled by user";

            *state = match std::mem::take(state) {
                BBImager::Flashing(inner) => {
                    inner.cancel_flashing.abort();

                    if inner.ctx.is_download() {
                        msg = "Download cancelled by user";
                    }
                    BBImager::FlashingCancel(inner.into())
                }
                BBImager::AppInfo(inner) => match inner.page {
                    OverlayData::Flashing(flashing_state) => {
                        flashing_state.cancel_flashing.abort();

                        if flashing_state.ctx.is_download() {
                            msg = "Download cancelled by user";
                        }

                        BBImager::AppInfo(OverlayState {
                            page: OverlayData::FlashingCancel(flashing_state.into()),
                            ..inner
                        })
                    }
                    _ => panic!("Unexpected message"),
                },
                _ => panic!("Unexpected message"),
            };

            return show_notification(msg.to_string());
        }
        BBImagerMessage::FlashFail(err) => {
            let mut msg = "Flashing failed";

            let logs =
                std::fs::read_to_string(helpers::log_file_path()).expect("Failed to read logs");
            let logs = iced::widget::text_editor::Content::with_text(&logs);

            *state = match std::mem::take(state) {
                BBImager::Flashing(inner) => {
                    if inner.ctx.is_download() {
                        msg = "Download failed";
                    }

                    BBImager::FlashingFail(crate::state::FlashingFailState::new(
                        inner.common,
                        inner.ctx,
                        err,
                        logs,
                    ))
                }
                BBImager::AppInfo(inner) => match inner.page {
                    OverlayData::Flashing(flashing_state) => {
                        if flashing_state.ctx.is_download() {
                            msg = "Download failed";
                        }

                        BBImager::AppInfo(OverlayState {
                            page: OverlayData::FlashingFail(crate::state::FlashingFailState::new(
                                flashing_state.common,
                                flashing_state.ctx,
                                err,
                                logs,
                            )),
                            ..inner
                        })
                    }
                    _ => panic!("Unexpected message"),
                },
                _ => panic!("Unexpected message"),
            };

            return show_notification(msg.to_string());
        }
        BBImagerMessage::FlashProgress(x) => match state {
            BBImager::Flashing(inner) => {
                inner.progress_update(x);
            }
            BBImager::AppInfo(inner) => match &mut inner.page {
                OverlayData::Flashing(flashing_state) => flashing_state.progress_update(x),
                _ => panic!("Unexpected message"),
            },
            // Debug build can be slow.
            _ => {}
        },
        BBImagerMessage::UiState(Message::FlashStart)
        | BBImagerMessage::UiState(Message::Retry) => {
            return state.start_flashing();
        }
        BBImagerMessage::FlashSuccess => {
            let mut msg = "Flashing finished successfully";

            *state = match std::mem::take(state) {
                BBImager::Flashing(inner) => {
                    if inner.ctx.is_download() {
                        msg = "Download finished successfully";
                    }
                    BBImager::FlashingSuccess(inner.into())
                }
                BBImager::AppInfo(inner) => match inner.page {
                    OverlayData::Flashing(flashing_state) => {
                        if flashing_state.ctx.is_download() {
                            msg = "Download finished successfully";
                        }

                        BBImager::AppInfo(OverlayState {
                            page: OverlayData::FlashingSuccess(flashing_state.into()),
                            ..inner
                        })
                    }
                    _ => panic!("Unexpected message"),
                },
                _ => panic!("Unexpected message"),
            };

            return show_notification(msg.to_string());
        }
        BBImagerMessage::UiState(Message::EditorEvent(evt)) => match evt {
            // The editors are read-only; only selection/scroll actions apply.
            iced::widget::text_editor::Action::Edit(_) => {}
            _ => match state {
                BBImager::FlashingFail(x) => x.inner.logs.perform(evt),
                BBImager::AppInfo(x) => x.inner.license.perform(evt),
                _ => panic!("Unexpected message"),
            },
        },
        BBImagerMessage::UiState(Message::GotoAppOptions) => {
            *state = BBImager::AppInfo(crate::state::OverlayState::new(
                std::mem::take(state).try_into().expect("Unexpected page"),
            ));

            return state.scroll_reset();
        }
        BBImagerMessage::UiState(Message::CopyToClipboard) => match state {
            BBImager::Review(inner) => {
                return helpers::FlashingInfo::json(inner.common.db.clone(), &inner.ctx)
                    .then(iced::clipboard::write);
            }
            BBImager::FlashingSuccess(inner) => {
                return helpers::FlashingInfo::json(inner.common.db.clone(), &inner.ctx)
                    .then(iced::clipboard::write);
            }
            BBImager::FlashingFail(inner) => {
                return iced::clipboard::write(inner.inner.logs.text());
            }
            _ => unreachable!(),
        },
        BBImagerMessage::UiState(Message::SelectFileDest(x)) => {
            return Task::done(BBImagerMessage::SelectFileDest(x));
        }
        BBImagerMessage::UiState(Message::ResolveImage(k, v)) => {
            state.common_mut().img_handle_cache.insert(k, v)
        }
        BBImagerMessage::DbInitSuccess => {
            let db = state.common().db.clone();
            let downloader = state.common().downloader.clone();

            let config_fetch_task = Task::future(blocking_future(move || {
                let configs = db.remote_configs().unwrap();
                let tasks = configs.into_iter().map(move |(i, u)| {
                    let dc = downloader.clone();
                    Task::perform(
                        async move {
                            let res = dc.download_json_no_cache(u).await?;
                            Ok((i, res))
                        },
                        |x: std::io::Result<(i64, bb_config::config::Config)>| match x {
                            Ok(y) => BBImagerMessage::ExtendConfig(y),
                            Err(e) => {
                                tracing::error!("Failed to fetch config: {e}");
                                BBImagerMessage::Null
                            }
                        },
                    )
                });
                iced::Task::batch(tasks)
            }))
            .then(std::convert::identity);

            let board_icon_task = state.common().fetch_board_images();
            let board_refresh_task = if let BBImager::ChooseBoard(x) = state {
                x.refresh_board_list()
            } else {
                Task::none()
            };

            return Task::batch([board_icon_task, config_fetch_task, board_refresh_task]);
        }
        BBImagerMessage::Null | BBImagerMessage::UiState(Message::Null) => {}
        _ => unimplemented!(),
    }

    Task::none()
}

fn show_notification(msg: String) -> Task<BBImagerMessage> {
    Task::future(async move {
        let res = helpers::show_notification(msg).await;
        tracing::debug!("Notification response {res:?}");
        BBImagerMessage::Null
    })
}
