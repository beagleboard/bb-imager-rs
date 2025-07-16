//! Global GUI Messages

use std::{borrow::Cow, collections::HashSet};

use iced::Task;

use crate::{
    BBImager,
    helpers::{self, BoardImage, Destination, FlashingCustomization, ProgressBarState},
    pages::Screen,
};

#[derive(Debug, Clone)]
pub(crate) enum BBImagerMessage {
    ExtendConfig(bb_config::Config),
    ResolveRemoteSubitemItem {
        item: Vec<bb_config::config::OsListItem>,
        target: Vec<usize>,
    },
    SelectBoard(usize),
    SelectImage(BoardImage),
    SelectLocalImage(bb_config::config::Flasher),
    SelectPort(Destination),
    ProgressBar(ProgressBarState),
    Destinations(Vec<Destination>),
    RefreshConfig,
    Reset,

    StartFlashing,
    StartFlashingWithoutConfiguraton,
    CancelFlashing,
    StopFlashing(ProgressBarState),
    UpdateFlashConfig(FlashingCustomization),

    /// Actions for App Settings
    UpdateSettings(crate::persistance::AppSettings),

    /// Write button was pressed
    WriteBtn,

    OpenUrl(Cow<'static, str>),

    /// Messages to ignore
    Null,

    /// Navigation
    ///
    /// Clear page stack and switch to new page
    SwitchScreen(Screen),
    /// Replace current page with new page
    ReplaceScreen(Screen),
    /// Push new page to the stack
    PushScreen(Screen),
    /// Pop page from stack
    PopScreen,

    /// Customization
    ///
    /// Save customization to disk
    SaveCustomization,
    /// Drop any customization changes that have not been saved
    CancelCustomization,
    /// Reset customization to default state
    ResetCustomization,
}

pub(crate) fn update(state: &mut BBImager, message: BBImagerMessage) -> Task<BBImagerMessage> {
    match message {
        BBImagerMessage::ExtendConfig(c) => {
            state.boards.merge(c);
            tracing::debug!("Update Config: {:#?}", state.boards);
            return state.fetch_board_images();
        }
        BBImagerMessage::RefreshConfig => {
            state.boards = Default::default();
            return helpers::refresh_config_task(state.downloader.clone(), &state.boards);
        }
        BBImagerMessage::ResolveRemoteSubitemItem { item, target } => {
            state.boards.resolve_remote_subitem(item, &target);
        }
        BBImagerMessage::SelectBoard(x) => {
            // Reset any previously selected values
            state.selected_dst.take();
            state.selected_image.take();
            state.destinations.clear();
            state.customization.take();

            let os_images = state
                .boards
                .images(x, &[])
                .expect("Initial image list can never be None");

            let remote_image_jobs = state.fetch_remote_subitems(x, &[]);
            let icons: HashSet<url::Url> = os_images.iter().map(|(_, x)| x.icon()).collect();
            state.selected_board = Some(x);

            let jobs = icons.into_iter().map(|x| {
                let downloader = state.downloader.clone();
                let x_clone = x.clone();
                Task::perform(
                    async move { downloader.download_no_cache(x_clone, None).await },
                    move |p| match p {
                        Ok(_path) => BBImagerMessage::Null,
                        Err(e) => {
                            tracing::warn!("Failed to download image {x} with error {e}");
                            BBImagerMessage::Null
                        }
                    },
                )
            });

            // Close Board selection page
            state.screen.pop();

            return Task::batch(jobs.chain([remote_image_jobs]));
        }
        BBImagerMessage::ProgressBar(x) => {
            if let Some(screen) = state.screen.pop() {
                match screen {
                    Screen::Flashing(s) => state.screen.push(Screen::Flashing(s.update(x))),
                    _ => state.screen.push(screen),
                }
            }
        }
        BBImagerMessage::SelectImage(x) => {
            tracing::info!("Selected Image: {}", x);
            state.selected_image = Some(x);
            state.screen.clear();
            state.screen.push(Screen::Home);
            state.customization = Some(state.config());
        }
        BBImagerMessage::SelectLocalImage(flasher) => {
            let extensions = helpers::file_filter(flasher);
            return Task::perform(
                async move {
                    rfd::AsyncFileDialog::new()
                        .add_filter("image", extensions)
                        .pick_file()
                        .await
                        .map(|x| x.path().to_path_buf())
                },
                move |x| match x {
                    Some(y) => BBImagerMessage::SelectImage(helpers::BoardImage::local(y, flasher)),
                    None => BBImagerMessage::Null,
                },
            );
        }
        BBImagerMessage::SelectPort(x) => {
            state.selected_dst = Some(x);
            state.screen.pop();
        }
        BBImagerMessage::Reset => {
            state.selected_dst.take();
            state.selected_image.take();
            state.selected_board.take();
            state.destinations.clear();
        }
        BBImagerMessage::SwitchScreen(x) => {
            state.screen.clear();
            return state.push_page(x);
        }
        BBImagerMessage::ReplaceScreen(x) => {
            state.screen.pop();
            return state.push_page(x);
        }
        BBImagerMessage::PushScreen(x) => {
            tracing::debug!("Push Page: {:?}", x);
            return state.push_page(x);
        }
        BBImagerMessage::PopScreen => {
            tracing::debug!("Pop screen");
            state.screen.pop();
        }
        BBImagerMessage::CancelFlashing => {
            if let Some(task) = state.cancel_flashing.take() {
                task.abort();
            }

            match state.screen.last().unwrap() {
                Screen::Flashing(s) => {
                    if let Some(y) = s.progress().cancel() {
                        return Task::done(BBImagerMessage::StopFlashing(y));
                    }
                }
                _ => unreachable!(),
            }
        }
        BBImagerMessage::WriteBtn => {
            let skip_confirmation = state.app_settings().skip_confirmation == Some(true);
            return match state.customization() {
                Some(x) if x.need_confirmation() && !skip_confirmation => {
                    state.push_page(Screen::FlashingConfirmation)
                }
                Some(x) => state.start_flashing(Some(x.clone())),
                None => state.start_flashing(None),
            };
        }
        BBImagerMessage::StartFlashing => {
            return state.start_flashing(state.customization.clone());
        }
        BBImagerMessage::StartFlashingWithoutConfiguraton => {
            return state.start_flashing(None);
        }
        BBImagerMessage::StopFlashing(x) => {
            let _ = state.cancel_flashing.take();
            let content = x.content();

            let progress_task = Task::done(BBImagerMessage::ProgressBar(x));
            let notification_task = Task::future(async move {
                let res = helpers::show_notification(content).await;

                tracing::debug!("Notification response {res:?}");
                BBImagerMessage::Null
            });

            return Task::batch([progress_task, notification_task]);
        }
        BBImagerMessage::Destinations(x) => {
            if !state.is_destionation_selectable() {
                assert_eq!(x.len(), 1);
                state.selected_dst = Some(x[0].clone());
            }
            state.destinations = x;
        }
        BBImagerMessage::UpdateFlashConfig(x) => state.customization = Some(x),
        BBImagerMessage::OpenUrl(x) => {
            return Task::future(async move {
                let res = webbrowser::open(&x);
                tracing::debug!("Open Url Resp {res:?}");
                BBImagerMessage::Null
            });
        }
        BBImagerMessage::SaveCustomization => {
            match state.customization.clone().unwrap() {
                FlashingCustomization::LinuxSd(c) => state.app_config.update_sd_customization(c),
                FlashingCustomization::Bcf(c) => state.app_config.update_bcf_customization(c),
                _ => {}
            }

            let config = state.app_config.clone();

            // Since we have a cache of config, no need to wait for disk persistance.
            state.screen.pop();

            return Task::future(async move {
                if let Err(e) = config.save().await {
                    tracing::error!("Failed to save config: {e}");
                }
                BBImagerMessage::Null
            });
        }
        BBImagerMessage::UpdateSettings(s) => {
            state.app_config.update_app_settings(s);

            let config = state.app_config.clone();
            return Task::future(async move {
                if let Err(e) = config.save().await {
                    tracing::error!("Failed to save config: {e}");
                }
                BBImagerMessage::Null
            });
        }
        BBImagerMessage::ResetCustomization => {
            state.customization = Some(state.customization.clone().unwrap().reset());
        }
        BBImagerMessage::CancelCustomization => {
            state.screen.pop();
            if state.customization.is_some() {
                state.customization = Some(state.config());
            }
        }
        BBImagerMessage::Null => {}
    };

    Task::none()
}
