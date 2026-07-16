use iced::{
    Element,
    widget::{self, button},
};

use crate::{
    BBImagerMessage, constants,
    state::FlashingFailState,
    ui::helpers::{VIEW_COL_PADDING, page_type1, progress_finish_view, selectable_text},
};

pub(crate) fn view(state: &FlashingFailState) -> Element<'_, BBImagerMessage> {
    page_type1(
        info_view(state),
        progress_finish_view("Failed", constants::DANGER, &state.err),
        [
            button("Flash New")
                .style(widget::button::danger)
                .on_press(BBImagerMessage::Restart),
            button("Retry")
                .style(widget::button::primary)
                .on_press(BBImagerMessage::Retry),
        ],
    )
}

pub(crate) fn info_view(state: &FlashingFailState) -> Element<'_, BBImagerMessage> {
    widget::column![
        widget::text("Logs").size(28).font(constants::FONT_BOLD),
        widget::rule::horizontal(2),
        selectable_text(&state.logs)
    ]
    .spacing(8)
    .padding(VIEW_COL_PADDING)
    .into()
}
