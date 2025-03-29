//! Global GUI Messages

use std::borrow::Cow;

use crate::{
    helpers::{BoardImage, Boards, Destination, FlashingCustomization, ProgressBarState},
    pages::Screen,
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
    SelectLocalImage(bb_config::config::Flasher),
    SelectPort(Destination),
    ProgressBar(ProgressBarState),
    Destinations(Vec<Destination>),
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
