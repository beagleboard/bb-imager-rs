//! First-run notice for sandboxed (Flatpak/snap) installs.
//!
//! Sandboxes cannot install udev rules, so a user asked to flash a board
//! would otherwise hit a permission wall later. On the first launch in a
//! sandbox this page points them at the rules file and the reload command.

use iced::widget::{self, button, text};
use iced::{Center, Element, Fill};

use crate::helpers::page_type3;
use crate::{Message, constants};

/// The URL of the shipped udev rules, as installed from a regular package.
fn udev_rules_url() -> url::Url {
    url::Url::parse(
        "https://github.com/beagleboard/bb-imager-rs/blob/main/bb-imager-gui/assets/packages/linux/udev/10-beagle.rules",
    )
    .unwrap()
}

#[derive(Debug)]
pub struct State {
    pub(crate) _private: (),
}

impl State {
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

pub fn view<'a>(state: &'a State, scroll_id: widget::Id) -> Element<'a, Message> {
    let _ = state;

    let col = widget::column![
        widget::svg(constants::INFO_ICON.clone())
            .height(64)
            .width(64),
        text("Missing udev permissions")
            .size(24)
            .font(constants::FONT_BOLD),
        text(
            "This copy of Beagle Imaging Utility runs sandboxed (Flatpak or snap), so it \
             cannot install the udev rules needed to flash Beagle boards.\n\nCopy the rules \
             file to /etc/udev/rules.d/ and run udevadm control --reload to grant card access.",
        ),
    ]
    .align_x(Center)
    .spacing(16);

    let view = widget::scrollable(
        widget::container(col)
            .align_x(Center)
            .align_y(Center)
            .height(Fill),
    )
    .id(scroll_id)
    .into();

    page_type3(
        view,
        [
            button("View Documentation")
                .style(widget::button::primary)
                .on_press(Message::OpenUrl(udev_rules_url())),
            button("Got it").on_press(Message::DismissSandboxNotice),
        ],
    )
}