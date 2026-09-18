use iced::Element;
use iced::widget::{self, button};

use crate::state::FlashingCancelState;
use crate::ui::helpers::{board_view_pane, page_type1, progress_finish_view};
use crate::{BBImagerMessage, constants};

pub(crate) fn cancel(state: &FlashingCancelState) -> Element<'_, BBImagerMessage> {
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
