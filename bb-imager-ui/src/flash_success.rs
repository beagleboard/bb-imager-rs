use std::sync::Arc;

use iced::{Element, widget};

use crate::board_selection::BoardDetails;
use crate::helpers::{board_details_pane, page_type1, progress_finish_view};
use crate::{Message, constants};

#[derive(Debug)]
pub struct State {
    /// Whether the image was written to a file rather than to a device.
    pub is_download: bool,
    pub board: BoardDetails,
}

pub fn view<'a>(
    cache: &'a bb_iced_widgets::cached_icon::Cache<Arc<url::Url>>,
    state: &'a State,
    scroll_id: widget::Id,
) -> Element<'a, Message> {
    let msg = if state.is_download {
        "Successfully Downloaded Image"
    } else {
        "Successfully Flashed Image"
    };

    page_type1(
        board_details_pane(cache, &state.board, &scroll_id),
        progress_finish_view("100%", constants::CHECK_MARK_GREEN, msg),
        [widget::button("Flash Another")
            .style(widget::button::primary)
            .on_press(Message::Restart)],
    )
}
