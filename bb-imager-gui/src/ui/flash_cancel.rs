use iced::{
    Element,
    widget::{self, button},
};

use crate::{
    BBImagerMessage, constants,
    state::FlashingFinishState,
    ui::helpers::{board_view_pane, page_type1, progress_finish_view},
};

pub(crate) fn view(state: &FlashingFinishState) -> Element<'_, BBImagerMessage> {
    page_type1(
        info_view(state),
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

pub(crate) fn info_view(state: &FlashingFinishState) -> Element<'_, BBImagerMessage> {
    board_view_pane(&state.selected_board, &state.common)
}
