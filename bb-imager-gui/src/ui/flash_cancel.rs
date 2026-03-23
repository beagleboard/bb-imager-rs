use iced::{
    Element,
    widget::{self, button},
};

use crate::{
    BBImagerMessage, constants,
    state::FlashingFinishState,
    ui::helpers::{CircleBar, VIEW_COL_PADDING, board_view_pane, page_type1},
};

pub(crate) fn view(state: &FlashingFinishState) -> Element<'_, BBImagerMessage> {
    page_type1(
        info_view(state),
        progress_view(),
        [button("Restart")
            .style(widget::button::danger)
            .on_press(BBImagerMessage::Restart)],
    )
}

pub(crate) fn progress_view() -> Element<'static, BBImagerMessage> {
    widget::column![
        CircleBar::new("Cancelled", 10.0, constants::DANGER),
        widget::text("Flashing Cancelled by the user")
    ]
    .align_x(iced::Center)
    .padding(VIEW_COL_PADDING)
    .into()
}

pub(crate) fn info_view(state: &FlashingFinishState) -> Element<'_, BBImagerMessage> {
    board_view_pane(state.selected_board(), &state.common)
}
