use iced::Element;
use iced::widget::{self, button};

use crate::state::FlashingFinishState;
use crate::ui::helpers::{board_view_pane, page_type1, progress_finish_view};
use crate::{BBImagerMessage, constants};

pub(crate) fn cancel(state: &FlashingFinishState) -> Element<'_, BBImagerMessage> {
    page_type1(
        board_view_pane(&state.selected_board, &state.common),
        progress_finish_view(
            "Cancelled",
            constants::DANGER,
            "Flashing Cancelled by the user",
        ),
        [button("Restart")
            .style(widget::button::danger)
            .on_press(BBImagerMessage::Restart)],
    )
}

pub(crate) fn success(state: &FlashingFinishState) -> Element<'_, BBImagerMessage> {
    let msg = if state.is_download {
        "Successfully Downloaded Image"
    } else {
        "Successfully Flashed Image"
    };

    page_type1(
        board_view_pane(&state.selected_board, &state.common),
        progress_finish_view("100%", constants::CHECK_MARK_GREEN, msg),
        [button("Flash Another")
            .style(widget::button::primary)
            .on_press(BBImagerMessage::Restart)],
    )
}
