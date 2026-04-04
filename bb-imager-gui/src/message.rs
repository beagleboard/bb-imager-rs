//! Global GUI Messages

use iced::Task;

use crate::{
    BBImager, helpers,
    state::{OverlayData, OverlayState},
};

#[derive(Debug, Clone)]
pub(crate) enum BBImagerMessage {
    /// Messages to ignore
    Null,

    /// Config related options
    ExtendConfig((url::Url, bb_config::Config)),
    ResolveRemoteSubitemItem {
        item: Vec<bb_config::config::OsListItem>,
        target: i64,
    },

    /// A new version of application is available
    UpdateAvailable(semver::Version),

    /// Select a board by index. Can only be used in Board selection page.
    UpdateBoardList(Vec<crate::db::BoardListItem>),
    SelectBoardById(i64),
    SelectBoard(crate::db::Board),

    /// ChooseOs Page
    UpdateOsList((Vec<helpers::OsImageItem>, Option<i64>)),
    SelectOs(helpers::OsImageId),
    SelectLocalOs(helpers::BoardImage),
    SelectRemoteOs((crate::db::OsImage, bb_config::config::Flasher)),
    GotoOsListParent,

    /// Choose Destination page
    SelectDest(helpers::Destination),
    SelectFileDest(String),
    DestinationFilter(bool),

    // Customization Page
    UpdateFlashConfig(crate::helpers::FlashingCustomization),
    ResetFlashingConfig,

    // Review Page
    FlashStart,

    // Flashing Page
    FlashProgress(bb_flasher::DownloadFlashingStatus),
    FlashSuccess,
    FlashCancel,
    FlashFail(String),

    // Reset to start from beginning.
    Restart,

    /// Open URL in browser
    OpenUrl(url::Url),

    /// Next button pressed
    Next,
    /// Back button pressed
    Back,

    /// Add image to cache
    ResolveImage(url::Url, std::path::PathBuf),
    ResolveImages(Vec<(url::Url, std::path::PathBuf)>),
    // Download images which have not already been downloaded
    FilterResolveImages(Vec<url::Url>),

    /// Update destinations
    Destinations(Vec<helpers::Destination>),

    /// Read-only editor
    EditorEvent(iced::widget::text_editor::Action),

    /// Show application information
    AppInfo,

    /// Copy text to clipboard.
    CopyToClipboard(String),

    /// DB Ops
    DbInitSuccess,

    /// Search
    UpdateSearchText(String),
}

pub(crate) fn update(state: &mut BBImager, message: BBImagerMessage) -> Task<BBImagerMessage> {
    match message {
        BBImagerMessage::SelectBoardById(id) => {
            let db = state.common().db.clone();
            return Task::future(async move {
                let b = db.board_by_id(id).await.expect("Incorrect board id");
                BBImagerMessage::SelectBoard(b)
            });
        }
        BBImagerMessage::UpdateBoardList(boards) => {
            // Update board list only if still on that page
            match state {
                BBImager::ChooseBoard(x) => {
                    x.boards = boards;
                }
                BBImager::AppInfo(overlay_state) => match &mut overlay_state.page {
                    OverlayData::ChooseBoard(x) => x.boards = boards,
                    _ => panic!("Unexpected message"),
                },
                _ => panic!("Unexpected message"),
            }
        }
        BBImagerMessage::SelectBoard(b) => match state {
            BBImager::ChooseBoard(inner) => {
                inner.selected_board = Some(b);
            }
            BBImager::AppInfo(overlay_state) => match &mut overlay_state.page {
                OverlayData::ChooseBoard(inner) => inner.selected_board = Some(b),
                _ => panic!("Unexpected message"),
            },
            _ => panic!("Unexpected message"),
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
        BBImagerMessage::SelectOs(id) => match state {
            BBImager::ChooseOs(inner) => match id {
                helpers::OsImageId::Format => {
                    inner.selected_image = Some((id, helpers::BoardImage::format()))
                }
                helpers::OsImageId::Local(flasher) => {
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
                helpers::OsImageId::OsImage(id) => {
                    let db = inner.common.db.clone();
                    let flasher = inner.flasher;
                    return Task::perform(async move { db.os_image_by_id(id).await }, move |x| {
                        match x {
                            Ok(i) => BBImagerMessage::SelectRemoteOs((i, flasher)),
                            Err(e) => {
                                tracing::error!("Failed to get os image {e}");
                                BBImagerMessage::Null
                            }
                        }
                    });
                }
                helpers::OsImageId::OsSublist(id) => {
                    let board_id = inner.selected_board.id;
                    return Task::batch([
                        state.refresh_image_list(board_id, Some(id)),
                        state.resolve_remote_sublists(board_id, Some(id)),
                    ]);
                }
            },
            _ => panic!("Unexpected message"),
        },
        BBImagerMessage::SelectRemoteOs((image, flasher)) => match state {
            BBImager::ChooseOs(inner) => {
                inner.selected_image = Some((
                    helpers::OsImageId::OsImage(image.id),
                    helpers::BoardImage::remote(image, flasher, inner.downloader().clone()),
                ));
            }
            BBImager::AppInfo(overlay_state) => match &mut overlay_state.page {
                OverlayData::ChooseOs(inner) => {
                    inner.selected_image = Some((
                        helpers::OsImageId::OsImage(image.id),
                        helpers::BoardImage::remote(image, flasher, inner.downloader().clone()),
                    ));
                }
                _ => panic!("Unexpected message"),
            },
            _ => panic!("Unexpected message"),
        },
        BBImagerMessage::SelectLocalOs(image) => match state {
            BBImager::ChooseOs(inner) => {
                inner.selected_image = Some((helpers::OsImageId::Local(image.flasher()), image))
            }
            _ => panic!("Unexpected message"),
        },
        BBImagerMessage::OpenUrl(x) => {
            return Task::future(async move {
                let res = webbrowser::open(x.as_str());
                tracing::debug!("Open Url Resp {res:?}");
                BBImagerMessage::Null
            });
        }
        BBImagerMessage::Next => return state.next(),
        BBImagerMessage::Back => return state.back(),
        BBImagerMessage::ResolveImage(k, v) => state.image_cache_insert(k, v),
        BBImagerMessage::ResolveImages(x) => state.image_cache_extend(x),
        BBImagerMessage::FilterResolveImages(x) => {
            let iter = x
                .into_iter()
                .filter(|x| !state.common().img_handle_cache.contains(x));
            return helpers::fetch_images(&state.common().downloader, iter);
        }
        BBImagerMessage::ExtendConfig((u, c)) => {
            tracing::debug!("Update Config: {:#?}", c);

            let db = state.common().db.clone();
            let db_task = Task::perform(
                async move {
                    db.add_config(c).await?;
                    db.remote_config_fetched(u).await
                },
                |x| {
                    if let Err(e) = x {
                        tracing::error!("Failed to merge config {e}");
                    }
                    BBImagerMessage::Null
                },
            );

            let tail_tasks = if let BBImager::ChooseBoard(inner) = state {
                // If we are in ChooseBoard page, update the board list
                Task::batch([
                    inner.common.fetch_board_images(),
                    inner.refresh_board_list(),
                ])
            } else {
                state.common().fetch_board_images()
            };

            // We want fetch board images to run after the config has been added
            return db_task.chain(tail_tasks);
        }
        BBImagerMessage::ResolveRemoteSubitemItem { item, target } => {
            let db = state.common().db.clone();
            let tail = match &state {
                BBImager::ChooseOs(inner) => Task::batch([
                    state.refresh_image_list(inner.selected_board.id, inner.pos),
                    state.refresh_image_icons(inner.selected_board.id),
                ]),
                _ => Task::none(),
            };

            return Task::future(async move {
                db.os_remote_sublist_resolve(target, &item).await.unwrap();
                BBImagerMessage::Null
            })
            .chain(tail);
        }
        BBImagerMessage::UpdateAvailable(x) => {
            return show_notification(format!("A new version of application is available {}", x));
        }
        BBImagerMessage::GotoOsListParent => match state {
            BBImager::ChooseOs(inner) => {
                let db = inner.common.db.clone();
                let curpos = inner.pos.unwrap();
                let board_id = inner.selected_board.id;
                return Task::perform(
                    async move {
                        let id = db.os_sublist_parent(curpos).await.unwrap();
                        let imgs = db.os_image_items(board_id, id).await.unwrap();

                        (imgs, id)
                    },
                    BBImagerMessage::UpdateOsList,
                );
            }
            _ => panic!("Unexpected message"),
        },
        BBImagerMessage::Destinations(x) => {
            if let BBImager::ChooseDest(inner) = state
                && x != inner.destinations
            {
                inner.destinations = x;
            }
        }
        BBImagerMessage::SelectDest(x) => match state {
            BBImager::ChooseDest(inner) => {
                inner.selected_dest = Some(x);
            }
            _ => panic!("Unexpected message"),
        },
        BBImagerMessage::SelectFileDest(x) => {
            return Task::perform(
                async move {
                    rfd::AsyncFileDialog::new()
                        .set_file_name(x)
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
        BBImagerMessage::DestinationFilter(x) => match state {
            BBImager::ChooseDest(inner) => {
                inner.filter_destination = x;
            }
            _ => panic!("Unexpected message"),
        },
        BBImagerMessage::UpdateFlashConfig(x) => match state {
            BBImager::Customize(inner) => {
                inner.customization = x;
            }
            _ => panic!("Unexpected message"),
        },
        BBImagerMessage::ResetFlashingConfig => match state {
            BBImager::Customize(inner) => {
                inner.customization.reset();
            }
            _ => panic!("Unexpected message"),
        },
        BBImagerMessage::FlashCancel => {
            let mut msg = "Flashing cancelled by user";

            *state = match std::mem::take(state) {
                BBImager::Flashing(inner) => {
                    inner.cancel_flashing.abort();

                    if inner.is_download {
                        msg = "Download cancelled by user";
                    }
                    BBImager::FlashingCancel(inner.into())
                }
                BBImager::AppInfo(inner) => match inner.page {
                    OverlayData::Flashing(flashing_state) => {
                        flashing_state.cancel_flashing.abort();

                        if flashing_state.is_download {
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
        BBImagerMessage::Restart => {
            return state.restart();
        }
        BBImagerMessage::FlashFail(err) => {
            let mut msg = "Flashing failed";

            let logs =
                std::fs::read_to_string(helpers::log_file_path()).expect("Failed to read logs");
            let logs = iced::widget::text_editor::Content::with_text(&logs);

            *state = match std::mem::take(state) {
                BBImager::Flashing(inner) => {
                    if inner.is_download {
                        msg = "Download failed";
                    }

                    BBImager::FlashingFail(crate::state::FlashingFailState {
                        common: inner.common,
                        err,
                        logs,
                    })
                }
                BBImager::AppInfo(inner) => match inner.page {
                    OverlayData::Flashing(flashing_state) => {
                        if flashing_state.is_download {
                            msg = "Download failed";
                        }

                        BBImager::AppInfo(OverlayState {
                            page: OverlayData::FlashingFail(crate::state::FlashingFailState {
                                common: flashing_state.common,
                                err,
                                logs,
                            }),
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
            _ => panic!("Unexpected message"),
        },
        BBImagerMessage::FlashStart => {
            return state.start_flashing();
        }
        BBImagerMessage::FlashSuccess => {
            let mut msg = "Flashing finished successfully";

            *state = match std::mem::take(state) {
                BBImager::Flashing(inner) => {
                    if inner.is_download {
                        msg = "Download finished successfully";
                    }
                    BBImager::FlashingSuccess(inner.into())
                }
                BBImager::AppInfo(inner) => match inner.page {
                    OverlayData::Flashing(flashing_state) => {
                        if flashing_state.is_download {
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
        BBImagerMessage::EditorEvent(evt) => match evt {
            iced::widget::text_editor::Action::Edit(_) => {}
            _ => match state {
                BBImager::FlashingFail(x) => x.logs.perform(evt),
                BBImager::AppInfo(x) => x.license.perform(evt),
                _ => panic!("Unexpected message"),
            },
        },
        BBImagerMessage::AppInfo => {
            *state = BBImager::AppInfo(crate::state::OverlayState::new(
                std::mem::take(state).try_into().expect("Unexpected page"),
            ));

            return state.scroll_reset();
        }
        BBImagerMessage::CopyToClipboard(data) => {
            return iced::clipboard::write(data);
        }
        BBImagerMessage::DbInitSuccess => {
            let db = state.common().db.clone();
            let downloader = state.common().downloader.clone();
            let config_fetch_task = Task::future(async move { db.remote_configs().await.unwrap() })
                .then(move |configs| {
                    let dc = downloader.clone();
                    let tasks = configs.into_iter().map(move |x| {
                        let dc = dc.clone();
                        Task::perform(
                            async move {
                                let res = dc.clone().download_json_no_cache(x.clone()).await?;
                                Ok((x, res))
                            },
                            |x: std::io::Result<(url::Url, bb_config::config::Config)>| match x {
                                Ok(y) => BBImagerMessage::ExtendConfig(y),
                                Err(e) => {
                                    tracing::error!("Failed to fetch config: {e}");
                                    BBImagerMessage::Null
                                }
                            },
                        )
                    });
                    iced::Task::batch(tasks)
                });

            let db = state.common().db.clone();
            let downloader = state.common().downloader.clone();
            let board_icon_cache_task = Task::perform(
                async move {
                    db.board_icons()
                        .await
                        .unwrap()
                        .into_iter()
                        .filter_map(|x| {
                            let p = downloader.check_cache_from_url(x.clone())?;
                            Some((x, p))
                        })
                        .collect()
                },
                BBImagerMessage::ResolveImages,
            );

            let board_refresh_task = if let BBImager::ChooseBoard(x) = state {
                x.refresh_board_list()
            } else {
                Task::none()
            };

            return Task::batch([board_icon_cache_task, config_fetch_task, board_refresh_task]);
        }
        BBImagerMessage::UpdateSearchText(x) => match state {
            BBImager::ChooseBoard(inner) => {
                return inner.update_search(x);
            }
            _ => {}
        },
        BBImagerMessage::Null => {}
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
