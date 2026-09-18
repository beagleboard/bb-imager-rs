use std::sync::Arc;

use iced::{Element, widget};

use crate::board_selection::BoardDetails;
use crate::helpers::{board_details_pane, page_type1, progress_finish_view};
use crate::{Message, constants};

#[derive(Debug)]
pub struct State {
    pub board: BoardDetails,
}

pub fn view<'a>(
    cache: &'a bb_iced_widgets::cached_icon::Cache<Arc<url::Url>>,
    state: &'a State,
    scroll_id: widget::Id,
) -> Element<'a, Message> {
    page_type1(
        board_details_pane(cache, &state.board, &scroll_id),
        progress_finish_view(
            "Cancelled",
            constants::DANGER,
            "Flashing Cancelled by the user",
        ),
        [widget::button("Restart")
            .style(widget::button::danger)
            .on_press(Message::Restart)],
    )
}
