use iced::{Element, widget};

use crate::helpers::{SidebarEntry, VIEW_COL_PADDING, page_type4};
use crate::{Message, constants};

#[derive(Debug)]
pub struct State {
    pub destinations: Box<[Box<str>]>,
}

/// Formatting reports no progress and cannot be cancelled, so this page only
/// says what is being formatted.
pub fn view<'a>(state: &'a State, scroll_id: widget::Id) -> Element<'a, Message> {
    page_type4(
        SidebarEntry::FormatMedia,
        true,
        widget::scrollable(
            widget::column![
                widget::text("Formatting")
                    .font(constants::FONT_BOLD)
                    .size(26),
                widget::text("Do not disconnect the storage device!").style(widget::text::danger),
                widget::rule::horizontal(2),
                widget::text("Formatting...")
                    .font(constants::FONT_BOLD)
                    .style(widget::text::secondary),
            ]
            .extend(state.destinations.iter().map(|x| {
                widget::text(x.as_ref())
                    .style(widget::text::secondary)
                    .into()
            }))
            .padding(VIEW_COL_PADDING)
            .spacing(16),
        )
        .id(scroll_id)
        .into(),
    )
}
