//! Global GUI Messages

use std::sync::Arc;

use bb_imager_ui::{Message, SidebarEntry};
use iced::Task;

use bb_imager_ui::image_selection::ImageId;

use crate::BBImager;
use crate::helpers::{self, blocking_future};
use crate::state::{OverlayData, OverlayState};

#[derive(Debug, Clone)]
pub(crate) enum BBImagerMessage {
    UiState(bb_imager_ui::Message),

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
    UpdateOsList((Vec<bb_imager_ui::image_selection::ImageItem>, Option<i64>)),
    SelectLocalOs {
        path: Box<std::path::Path>,
        flasher: bb_config::config::Flasher,
        size: u64,
    },
    SelectRemoteOs((crate::db::OsImage, bb_config::config::Flasher)),

    /// Choose Destination page
    SelectDest(helpers::Destination),

    // Flashing Page
    FlashProgress(bb_flasher::DownloadFlashingStatus),
    FlashSuccess,
    FlashFail(String),

    // Download images which have not already been downloaded
    FilterResolveImages(Vec<Arc<url::Url>>),

    /// Update destinations
    Destinations(Box<[helpers::Destination]>),

    /// Copy text to clipboard.
    CopyToClipboard(String),

    /// DB Ops
    DbInitSuccess,
}

pub(crate) fn update(state: &mut BBImager, message: BBImagerMessage) -> Task<BBImagerMessage> {
    match message {
        BBImagerMessage::UiState(Message::SelectBoardById(id)) => {
            let db = state.common().db.clone();
            return Task::perform(
                blocking_future(move || db.board_by_id(id).expect("Incorrect board id")),
                BBImagerMessage::SelectBoard,
            );
        }
        BBImagerMessage::UpdateBoardList(boards) => {
            // Update board list only if still on that page
            match state {
                BBImager::ChooseBoard(x) => {
                    x.state.boards = boards;
                }
                BBImager::AppInfo(overlay_state) => {
                    if let OverlayData::ChooseBoard(x) = &mut overlay_state.page {
                        x.state.boards = boards
                    }
                }
                _ => {}
            }
        }
        BBImagerMessage::SelectBoard(b) => match state {
            BBImager::ChooseBoard(inner) => inner.select_board(b),
            BBImager::AppInfo(overlay_state) => {
                if let OverlayData::ChooseBoard(inner) = &mut overlay_state.page {
                    inner.select_board(b)
                }
            }
            _ => {}
        },
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
        BBImagerMessage::UiState(Message::SelectOs(id)) => match state {
            BBImager::ChooseOs(inner) => match id {
                ImageId::Format => inner.select_format_image(),
                ImageId::Local(flasher) => {
                    let extensions = helpers::file_filter(flasher);

                    return Task::perform(
                        async move {
                            let fpath = rfd::AsyncFileDialog::new()
                                .add_filter("image", extensions)
                                .pick_file()
                                .await
                                .map(|x| x.inner().to_path_buf().into())?;

                            let size = tokio::fs::metadata(&fpath).await.unwrap().len();

                            Some((fpath, size))
                        },
                        move |x| match x {
                            Some((path, size)) => BBImagerMessage::SelectLocalOs {
                                path,
                                flasher,
                                size,
                            },
                            None => BBImagerMessage::Null,
                        },
                    );
                }
                ImageId::OsImage(id) => {
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
                ImageId::OsSublist(id) => {
                    let board_id = inner.selected_board.id;
                    return Task::batch([
                        inner.resolve_remote_sublists(board_id, Some(id.0)),
                        inner.update_pos(Some(id.0), id.1),
                    ]);
                }
            },
            _ => panic!("Unexpected message"),
        },
        BBImagerMessage::SelectRemoteOs((image, flasher)) => match state {
            BBImager::ChooseOs(inner) => {
                inner.select_remote_image(image, flasher);
            }
            BBImager::AppInfo(overlay_state) => {
                if let OverlayData::ChooseOs(inner) = &mut overlay_state.page {
                    inner.select_remote_image(image, flasher);
                }
            }
            _ => {}
        },
        BBImagerMessage::SelectLocalOs {
            path,
            flasher,
            size,
        } => match state {
            BBImager::ChooseOs(inner) => inner.select_local_image(path, flasher, size),
            _ => panic!("Unexpected message"),
        },
        BBImagerMessage::UiState(Message::OpenUrl(x)) => {
            return Task::future(async move {
                let res = webbrowser::open(x.as_str());
                tracing::debug!("Open Url Resp {res:?}");
                BBImagerMessage::Null
            });
        }
        BBImagerMessage::UiState(Message::Next) => return state.next(),
        BBImagerMessage::UiState(Message::ResolveImage(k, v)) => state.image_cache_insert(k, v),
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
                    inner.common.refresh_image_icons(inner.selected_board.id),
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
        BBImagerMessage::UiState(Message::GotoOsListParent) => match state {
            BBImager::ChooseOs(inner) => {
                let db = inner.common.db.clone();
                let curpos = inner.state.pos.unwrap();
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
            _ => panic!("Unexpected message"),
        },
        BBImagerMessage::Destinations(x) => {
            if let BBImager::ChooseDest(inner) = state
                && x != inner.destinations
            {
                inner.update_destinations(x);
            }
        }
        BBImagerMessage::SelectDest(x) => match state {
            BBImager::ChooseDest(inner) => inner.select_dest(x),
            _ => panic!("Unexpected message"),
        },
        BBImagerMessage::UiState(Message::SelectDest(ident)) => match state {
            // Resolved by identity rather than position: the list is
            // re-enumerated every second, so a click landing after a refresh
            // must not retarget to whatever now sits at that index.
            BBImager::ChooseDest(inner) => {
                match inner
                    .destinations
                    .iter()
                    .find(|x| x.identifier().as_ref() == ident.as_ref())
                    .cloned()
                {
                    Some(dest) => inner.select_dest(dest),
                    None => tracing::warn!("Destination {ident} is gone; ignoring selection"),
                }
            }
            _ => panic!("Unexpected message"),
        },
        // The page derives this row and owns the suggested name, so there is
        // nothing to re-derive here.
        BBImagerMessage::UiState(Message::SelectFileDest(name)) => {
            return Task::perform(
                async move {
                    rfd::AsyncFileDialog::new()
                        .set_file_name(name.as_ref())
                        .save_file()
                        .await
                        .map(|x| x.inner().to_path_buf())
                },
                move |x| match x {
                    Some(y) => BBImagerMessage::SelectDest(helpers::Destination::LocalFile(y)),
                    None => BBImagerMessage::Null,
                },
            );
        }
        BBImagerMessage::UiState(Message::DestinationFilter(x)) => match state {
            BBImager::ChooseDest(inner) => {
                inner.state.filter_destination = x;
            }
            _ => panic!("Unexpected message"),
        },
        BBImagerMessage::UiState(Message::UpdateCustomization(x)) => match state {
            BBImager::Customize(inner) => {
                inner.state.customization = x;
            }
            _ => panic!("Unexpected message"),
        },
        BBImagerMessage::UiState(Message::Reset) => match state {
            BBImager::Customize(inner) => {
                // Reset through the persisted defaults, so platform-specific
                // ones (USB DHCP on macOS) survive.
                let default = crate::persistance::SdSysconfCustomization::default();
                inner.state.customization = match inner.state.customization {
                    bb_imager_ui::configuration::Customization::SysConfig(_) => {
                        bb_imager_ui::configuration::Customization::SysConfig(default.into())
                    }
                    bb_imager_ui::configuration::Customization::CloudInit(_) => {
                        bb_imager_ui::configuration::Customization::CloudInit(default.into())
                    }
                };
            }
            _ => panic!("Unexpected message"),
        },
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
        BBImagerMessage::UiState(Message::Restart) => {
            return state.restart();
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

                    BBImager::FlashingFail(crate::state::FlashingFailState::new(inner, err, logs))
                }
                BBImager::AppInfo(inner) => {
                    match inner.page {
                        OverlayData::Flashing(flashing_state) => {
                            if flashing_state.ctx.is_download() {
                                msg = "Download failed";
                            }

                            BBImager::AppInfo(OverlayState {
                                page: OverlayData::FlashingFail(
                                    crate::state::FlashingFailState::new(flashing_state, err, logs),
                                ),
                                ..inner
                            })
                        }
                        _ => panic!("Unexpected message"),
                    }
                }
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
            iced::widget::text_editor::Action::Edit(_) => {}
            _ => match state {
                BBImager::FlashingFail(x) => x.state.logs.perform(evt),
                BBImager::AppInfo(x) => x.state.license.perform(evt),
                _ => panic!("Unexpected message"),
            },
        },
        // Leave App Options for the page beneath, then jump from there.
        BBImagerMessage::UiState(Message::Goto(target))
            if matches!(state, BBImager::AppInfo(_)) =>
        {
            let BBImager::AppInfo(inner) = std::mem::take(state) else {
                unreachable!()
            };
            *state = inner.page.into();

            return update(state, BBImagerMessage::UiState(Message::Goto(target)));
        }
        BBImagerMessage::UiState(Message::Goto(SidebarEntry::AppOptions)) => {
            *state = BBImager::AppInfo(crate::state::OverlayState::new(
                std::mem::take(state).try_into().expect("Unexpected page"),
            ));

            return state.scroll_reset();
        }
        // Leaving a running flash would orphan its task.
        BBImagerMessage::UiState(Message::Goto(SidebarEntry::Hardware))
            if !matches!(state, BBImager::Flashing(_)) =>
        {
            let task = state.restart();
            return Task::batch([task, state.scroll_reset()]);
        }
        BBImagerMessage::UiState(Message::Goto(SidebarEntry::Software)) => {
            let page = match std::mem::take(state) {
                BBImager::ChooseOs(x) => x,
                BBImager::ChooseDest(x) => x.into(),
                BBImager::Customize(x) => x.into(),
                BBImager::Review(x) => x.into(),
                BBImager::FlashingFail(x) => x.into(),
                BBImager::FlashingSuccess(x) => x.into(),
                BBImager::FlashingCancel(x) => x.into(),
                _ => unreachable!(),
            };
            let board_id = page.selected_board.id;
            let tasks = Task::batch([
                page.refresh_image_list(),
                page.common.refresh_image_icons(board_id),
            ]);
            *state = BBImager::ChooseOs(page);

            return Task::batch([tasks, state.scroll_reset()]);
        }
        BBImagerMessage::UiState(Message::Goto(SidebarEntry::Storage)) => {
            *state = BBImager::ChooseDest(match std::mem::take(state) {
                BBImager::ChooseDest(x) => x,
                BBImager::Customize(x) => x.into(),
                BBImager::Review(x) => x.into(),
                BBImager::FlashingFail(x) => x.into(),
                BBImager::FlashingSuccess(x) => x.into(),
                BBImager::FlashingCancel(x) => x.into(),
                _ => unreachable!(),
            });

            return state.scroll_reset();
        }
        BBImagerMessage::UiState(Message::Goto(SidebarEntry::Modify)) => {
            *state = BBImager::Customize(match std::mem::take(state) {
                BBImager::Customize(x) => x,
                BBImager::Review(x) => x.into(),
                BBImager::FlashingFail(x) => x.into(),
                BBImager::FlashingSuccess(x) => x.into(),
                BBImager::FlashingCancel(x) => x.into(),
                _ => unreachable!(),
            });

            return state.scroll_reset();
        }
        BBImagerMessage::UiState(Message::Goto(SidebarEntry::Review)) => {
            *state = BBImager::Review(match std::mem::take(state) {
                BBImager::Review(x) => x,
                BBImager::FlashingFail(x) => x.into(),
                BBImager::FlashingSuccess(x) => x.into(),
                BBImager::FlashingCancel(x) => x.into(),
                _ => unreachable!(),
            });

            return state.scroll_reset();
        }
        BBImagerMessage::UiState(Message::Goto(SidebarEntry::Write))
            if matches!(
                state,
                BBImager::Flashing(_)
                    | BBImager::FlashingFail(_)
                    | BBImager::FlashingSuccess(_)
                    | BBImager::FlashingCancel(_)
            ) =>
        {
            return state.scroll_reset();
        }
        // The sidebar only enables the steps handled above.
        BBImagerMessage::UiState(Message::Goto(_)) => unreachable!(),
        BBImagerMessage::CopyToClipboard(data) => {
            return iced::clipboard::write(data);
        }
        BBImagerMessage::UiState(Message::CopyBoardConfig(id)) => {
            let db = state.common().db.clone();
            return Task::perform(
                blocking_future(move || db.os_board_json_by_id(id)),
                |x| match x {
                    Ok(b) => BBImagerMessage::CopyToClipboard(
                        serde_json::to_string_pretty(&b).expect("Device is always serializable"),
                    ),
                    Err(e) => {
                        tracing::error!("Failed to get board config: {e}");
                        BBImagerMessage::Null
                    }
                },
            );
        }
        BBImagerMessage::UiState(Message::CopyImageConfig(id)) => {
            let db = state.common().db.clone();
            return Task::perform(
                blocking_future(move || db.os_image_json_by_id(id)),
                |x| match x {
                    Ok(i) => BBImagerMessage::CopyToClipboard(
                        serde_json::to_string_pretty(&i).expect("OsImage is always serializable"),
                    ),
                    Err(e) => {
                        tracing::error!("Failed to get image config: {e}");
                        BBImagerMessage::Null
                    }
                },
            );
        }
        BBImagerMessage::DbInitSuccess => {
            let db = state.common().db.clone();
            let downloader = state.common().downloader.clone();

            let config_fetch_task = Task::future(blocking_future(move || {
                let configs = db.remote_configs().unwrap();
                let tasks = configs.into_iter().map(move |(i, u)| {
                    let dc = downloader.clone();
                    let u_clone = u.clone();
                    Task::perform(
                        async move {
                            let res = dc.download_json_no_cache(u_clone).await?;
                            Ok((i, res))
                        },
                        move |x: std::io::Result<(i64, bb_config::config::Config)>| match x {
                            Ok(y) => BBImagerMessage::ExtendConfig(y),
                            Err(e) => {
                                tracing::error!("Failed to fetch config: {e} {u}");
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
        BBImagerMessage::UiState(Message::UpdateSearchText(x)) => match state {
            BBImager::ChooseBoard(inner) => return inner.update_search(x),
            BBImager::ChooseOs(inner) => return inner.update_search(x),
            BBImager::ChooseDest(inner) => inner.update_search(x),
            _ => {}
        },
        BBImagerMessage::UiState(Message::UpdateInitFormat(f)) => {
            if let BBImager::ChooseOs(inner) = state
                && let Some(bb_imager_ui::image_selection::ImageDetails::Local {
                    init_format, ..
                }) = &mut inner.state.selected
            {
                *init_format = f;
            }
        }
        BBImagerMessage::Null | BBImagerMessage::UiState(Message::Null) => {}
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
