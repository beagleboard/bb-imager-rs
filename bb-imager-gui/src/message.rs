//! Global GUI Messages

use std::sync::Arc;

use bb_imager_ui::Message;
use iced::Task;

use bb_imager_ui::image_selection::ImageId;

use crate::{
    BBImager,
    helpers::{self, blocking_future},
    state::{OverlayData, OverlayState},
};

/// Arrow key navigation direction.
#[derive(Debug, Clone, Copy)]
pub(crate) enum NavDir {
    Up,
    Down,
}

#[derive(Debug, Clone)]
pub(crate) enum BBImagerMessage {
    UiState(bb_imager_ui::Message),

    /// Messages to ignore
    Null,

    /// Mouse cursor moved; feeds the double-click detector.
    MouseMoved(iced::Point),

    /// Left mouse button pressed; feeds the double-click detector.
    MouseClicked,

    /// Enter pressed: advance to the next step when a selection exists.
    KeyboardNext,

    /// Escape pressed: navigate to the previous step.
    KeyboardBack,

    /// Arrow key pressed: move the selection on a list page.
    KeyboardNavigate(NavDir),

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
    SelectLocalOs(helpers::BoardImage),
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
                ImageId::Format => inner.select_image(id, helpers::BoardImage::format()),
                ImageId::Local(flasher) => {
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
                            Some(y) => BBImagerMessage::SelectLocalOs(helpers::BoardImage::local(
                                y, flasher,
                            )),
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
                let id = ImageId::OsImage(image.id);
                let img =
                    helpers::BoardImage::remote(image, flasher, inner.common.downloader.clone());
                inner.select_image(id, img);
            }
            BBImager::AppInfo(overlay_state) => {
                if let OverlayData::ChooseOs(inner) = &mut overlay_state.page {
                    let id = ImageId::OsImage(image.id);
                    let img = helpers::BoardImage::remote(
                        image,
                        flasher,
                        inner.common.downloader.clone(),
                    );
                    inner.select_image(id, img);
                }
            }
            _ => {}
        },
        BBImagerMessage::SelectLocalOs(image) => match state {
            BBImager::ChooseOs(inner) => inner.select_image(ImageId::Local(image.flasher()), image),
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
        BBImagerMessage::UiState(Message::Back) => return state.back(),
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
        BBImagerMessage::UiState(Message::GotoAppInfo) => {
            *state = BBImager::AppInfo(crate::state::OverlayState::new(
                std::mem::take(state).try_into().expect("Unexpected page"),
            ));

            return state.scroll_reset();
        }
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
                && let Some((_, img)) = &mut inner.selected_image
            {
                img.update_init_format(f);
            }
        }
        BBImagerMessage::MouseMoved(pos) => {
            state.common_mut().clicks.record_cursor(pos);
        }
        BBImagerMessage::MouseClicked => {
            if let Some(page) = nav_page(state) {
                let is_double = state.common_mut().clicks.click(page);
                if is_double && state.can_next() {
                    return state.next();
                }
            }
        }
        BBImagerMessage::KeyboardNext => {
            if state.can_next() {
                return state.next();
            }
        }
        BBImagerMessage::KeyboardBack => {
            if state.can_back() {
                return state.back();
            }
        }
        BBImagerMessage::KeyboardNavigate(dir) => {
            if let Some(click) = navigate(state, dir) {
                return update(state, BBImagerMessage::UiState(click));
            }
        }
        BBImagerMessage::Null | BBImagerMessage::UiState(Message::Null) => {}
    }

    Task::none()
}

/// The page tag used to reject cross-page double clicks.
fn nav_page(state: &BBImager) -> Option<&'static str> {
    match state {
        BBImager::ChooseBoard(_) => Some("board"),
        BBImager::ChooseOs(_) => Some("os"),
        BBImager::ChooseDest(_) => Some("dest"),
        _ => None,
    }
}

/// Step the current selection index `dir`, seeding it at the ends when unset.
fn adjacent(idx: Option<usize>, len: usize, dir: NavDir) -> Option<usize> {
    if len == 0 {
        return None;
    }
    match (dir, idx) {
        (NavDir::Down, None) => Some(0),
        (NavDir::Down, Some(i)) if i + 1 < len => Some(i + 1),
        (NavDir::Up, None) => Some(len - 1),
        (NavDir::Up, Some(i)) if i > 0 => Some(i - 1),
        _ => None,
    }
}

/// Re-emits the select message a click would have produced after an arrow
/// key, so the list rows on the selection pages can be walked with the
/// keyboard (the rows are plain buttons, which ignore keys in iced 0.14).
fn navigate(state: &BBImager, dir: NavDir) -> Option<bb_imager_ui::Message> {
    match state {
        BBImager::ChooseBoard(inner) => {
            let len = inner.state.boards.len();
            let idx = inner
                .selected_board
                .as_ref()
                .and_then(|b| inner.state.boards.iter().position(|x| x.id == b.id));
            adjacent(idx, len, dir).map(|i| Message::SelectBoardById(inner.state.boards[i].id))
        }
        BBImager::ChooseOs(inner) => {
            let has_back = inner.state.pos.is_some();
            let len = inner.state.images.len();
            let idx = inner
                .selected_image
                .as_ref()
                .and_then(|(id, _)| inner.state.images.iter().position(|x| &x.id == id))
                .map(|i| i + usize::from(has_back));
            let row = adjacent(idx, len + usize::from(has_back), dir)?;
            if has_back && row == 0 {
                Some(Message::GotoOsListParent)
            } else {
                Some(Message::SelectOs(
                    inner.state.images[row - usize::from(has_back)].id,
                ))
            }
        }
        BBImager::ChooseDest(inner) => {
            let save_row = inner.state.image_file_name.is_some();
            let rows = inner.destinations.len() + usize::from(save_row);
            let idx = match inner.selected_dest.as_ref() {
                Some(helpers::Destination::LocalFile(_)) if save_row => {
                    Some(inner.destinations.len())
                }
                Some(dest) => inner
                    .destinations
                    .iter()
                    .position(|x| x.identifier().as_ref() == dest.identifier().as_ref()),
                None => None,
            };
            let row = adjacent(idx, rows, dir)?;
            if save_row && row == inner.destinations.len() {
                Some(Message::SelectFileDest(
                    inner.state.image_file_name.clone()?,
                ))
            } else {
                Some(Message::SelectDest(
                    inner.destinations[row]
                        .identifier()
                        .into_owned()
                        .into_boxed_str(),
                ))
            }
        }
        _ => None,
    }
}

fn show_notification(msg: String) -> Task<BBImagerMessage> {
    Task::future(async move {
        let res = helpers::show_notification(msg).await;
        tracing::debug!("Notification response {res:?}");
        BBImagerMessage::Null
    })
}
