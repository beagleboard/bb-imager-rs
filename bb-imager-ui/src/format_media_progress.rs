use iced::{Element, widget};

use crate::helpers::{SidebarEntry, VIEW_COL_PADDING, page_type4};
use crate::{Message, constants};

#[derive(Debug)]
pub struct State {
    pub destination: Box<str>,
}

/// Formatting reports no progress and cannot be cancelled, so this page only
/// says what is being formatted.
pub fn view<'a>(state: &'a State) -> Element<'a, Message> {
    page_type4(
        SidebarEntry::FormatMedia,
        true,
        widget::column![
            widget::text("Formatting")
                .font(constants::FONT_BOLD)
                .size(26),
            widget::text("Do not disconnect the storage device!").style(widget::text::danger),
            widget::rule::horizontal(2),
            widget::text(format!("Formatting {}...", state.destination))
                .font(constants::FONT_BOLD)
                .style(widget::text::secondary),
        ]
        .padding(VIEW_COL_PADDING)
        .spacing(16)
        .into(),
    )
}
