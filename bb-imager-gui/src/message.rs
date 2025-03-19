//! Global GUI Messages

use std::{borrow::Cow, collections::HashSet};

use crate::{
    helpers::{BoardImage, Boards, ProgressBarState},
    pages::{Screen, configuration::FlashingCustomization},
};

#[derive(Debug, Clone)]
pub(crate) enum BBImagerMessage {
    UpdateConfig(Boards),
    ResolveRemoteSubitemItem {
        item: Vec<bb_config::config::OsListItem>,
        target: Vec<usize>,
    },
    BoardSelected(usize),
    SelectImage(BoardImage),
    SelectLocalImage(bb_imager::Flasher),
    SelectPort(bb_imager::Destination),
    ProgressBar(ProgressBarState),
    Search(String),
    Destinations(HashSet<bb_imager::Destination>),
    Reset,

    StartFlashing,
    StartFlashingWithoutConfiguraton,
    CancelFlashing,
    StopFlashing(ProgressBarState),
    UpdateFlashConfig(FlashingCustomization),

    OpenUrl(Cow<'static, str>),

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
