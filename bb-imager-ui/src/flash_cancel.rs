use iced::{Element, widget};

use crate::helpers::{VIEW_COL_PADDING, page_type2};
use crate::{Message, constants};

pub fn view<'a>(scroll_id: widget::Id) -> Element<'a, Message> {
    page_type2(
        cancel_view(scroll_id),
        [widget::button("Restart")
            .style(widget::button::danger)
            .on_press(Message::Restart)],
    )
}

fn cancel_view<'a>(scroll_id: widget::Id) -> Element<'a, Message> {
    widget::scrollable(
        widget::column![
            widget::text("Write Cancelled")
                .size(28)
                .font(constants::FONT_BOLD),
            widget::text("Flashing Cancelled by the user").style(widget::text::warning)
        ]
        .spacing(8)
        .padding(VIEW_COL_PADDING),
    )
    .id(scroll_id)
    .into()
}
