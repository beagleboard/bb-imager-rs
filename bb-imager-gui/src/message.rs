//! Global GUI Messages

use iced::Task;

use crate::{BBImager, helpers};

#[derive(Debug, Clone)]
pub(crate) enum BBImagerMessage {
    /// Messages to ignore
    Null,

    ExtendConfig(bb_config::Config),
    ResolveRemoteSubitemItem {
        item: Vec<bb_config::config::OsListItem>,
        target: Vec<usize>,
    },

    /// A new version of application is available
    UpdateAvailable(semver::Version),

    /// Select a board by index. Can only be used in Board selection page.
    SelectBoard(usize),

    /// ChooseOs Page
    SelectOs(crate::OsImageId),
    SelectLocalOs((Vec<usize>, helpers::BoardImage)),
    GotoOsListParent,

    /// Choose Destination page
    SelectDest(helpers::Destination),
    SelectFileDest(String),

    // Customization Page
    UpdateFlashConfig(crate::helpers::FlashingCustomization),

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

    Destinations(Vec<helpers::Destination>),

    EditorEvent(iced::widget::text_editor::Action),

    AppInfo,
}

pub(crate) fn update(state: &mut BBImager, message: BBImagerMessage) -> Task<BBImagerMessage> {
    match message {
        BBImagerMessage::SelectBoard(id) => match state {
            BBImager::ChooseBoard(inner) => {
                inner.selected_board = Some(id);
            }
            _ => panic!("Unexpected message"),
        },
        BBImagerMessage::SelectOs(id) => match state {
            BBImager::ChooseOs(inner) => match id {
                crate::OsImageId::Format(_) => {
                    inner.selected_image = Some((id, helpers::BoardImage::format()))
                }
                crate::OsImageId::Local(parent) => {
                    let flasher = inner.flasher();
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
                            Some(y) => BBImagerMessage::SelectLocalOs((
                                parent,
                                helpers::BoardImage::local(y, flasher),
                            )),
                            None => BBImagerMessage::Null,
                        },
                    );
                }
                crate::OsImageId::Remote(target) => {
                    if let bb_config::config::OsListItem::Image(x) = inner.image(&target) {
                        inner.selected_image = Some((
                            crate::OsImageId::Remote(target),
                            helpers::BoardImage::remote(
                                x.clone(),
                                inner.flasher(),
                                inner.downloader().clone(),
                            ),
                        ))
                    } else {
                        inner.pos = target
                    }
                }
            },
            _ => panic!("Unexpected message"),
        },
        BBImagerMessage::SelectLocalOs((parent, image)) => match state {
            BBImager::ChooseOs(inner) => {
                inner.selected_image = Some((crate::OsImageId::Local(parent), image))
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
        BBImagerMessage::Back => state.back(),
        BBImagerMessage::ResolveImage(k, v) => state.image_cache_insert(k, v),
        BBImagerMessage::ExtendConfig(c) => {
            tracing::debug!("Update Config: {:#?}", c);
            state.boards_merge(c);
            return state.fetch_board_images();
        }
        BBImagerMessage::ResolveRemoteSubitemItem { item, target } => {
            state.resolve_remote_subitem(item, &target)
        }
        BBImagerMessage::UpdateAvailable(x) => {
            return Task::future(async move {
                let res = helpers::show_notification(format!(
                    "A new version of application is available {}",
                    x
                ))
                .await;
                tracing::debug!("Notification response {res:?}");
                BBImagerMessage::Null
            });
        }
        BBImagerMessage::GotoOsListParent => match state {
            BBImager::ChooseOs(inner) => {
                inner.pos.pop();
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
        BBImagerMessage::UpdateFlashConfig(x) => match state {
            BBImager::Customize(inner) => {
                inner.customization = x;
            }
            _ => panic!("Unexpected message"),
        },
        BBImagerMessage::FlashCancel => {
            *state = match std::mem::take(state) {
                BBImager::Flashing(inner) => {
                    inner.cancel_flashing.abort();

                    BBImager::FlashingCancel(crate::FlashingFinishState {
                        common: inner.common,
                        selected_board: inner.selected_board,
                    })
                }
                _ => panic!("Unexpected message"),
            };
        }
        BBImagerMessage::Restart => {
            state.restart();
        }
        BBImagerMessage::FlashFail(err) => {
            *state = match std::mem::take(state) {
                BBImager::Flashing(inner) => {
                    let logs = std::fs::read_to_string(helpers::log_file_path())
                        .expect("Failed to read logs");
                    let logs = iced::widget::text_editor::Content::with_text(&logs);

                    BBImager::FlashingFail(crate::FlashingFailState {
                        common: inner.common,
                        err,
                        logs,
                    })
                }
                _ => panic!("Unexpected message"),
            };
        }
        BBImagerMessage::FlashProgress(x) => match state {
            BBImager::Flashing(inner) => {
                inner.progress_update(x);
            }
            _ => panic!("Unexpected message"),
        },
        BBImagerMessage::FlashStart => {
            return state.start_flashing();
        }
        BBImagerMessage::FlashSuccess => {
            *state = match std::mem::take(state) {
                BBImager::Flashing(inner) => {
                    BBImager::FlashingSuccess(crate::FlashingFinishState {
                        common: inner.common,
                        selected_board: inner.selected_board,
                    })
                }
                _ => panic!("Unexpected message"),
            }
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
            *state = BBImager::AppInfo(crate::OverlayState::new(
                std::mem::take(state).try_into().expect("Unexpected page"),
            ));
        }
        _ => {}
    }

    Task::none()
}
