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
        board_view_pane(&state.selected_board, &state.common),
        progress_view(state),
        [button("Flash Another")
            .style(widget::button::primary)
            .on_press(BBImagerMessage::Restart)],
    )
}

fn progress_view(state: &FlashingFinishState) -> Element<'static, BBImagerMessage> {
    let msg = if state.is_download {
        "Successfully Downloaded Image"
    } else {
        "Successfully Flashed Image"
    };

    progress_finish_view("100%", constants::CHECK_MARK_GREEN, msg)
}
