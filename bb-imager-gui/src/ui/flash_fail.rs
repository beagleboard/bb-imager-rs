use iced::{
    Element,
    widget::{self, button},
};

use crate::{
    BBImagerMessage, FlashingFailState, constants,
    ui::helpers::{CircleBar, page_type1, selectable_text},
};

pub(crate) fn view(state: &FlashingFailState) -> Element<'_, BBImagerMessage> {
    page_type1(
        &state.common,
        info_view(state),
        progress_view(state),
        [button("Restart")
            .style(widget::button::danger)
            .on_press(BBImagerMessage::Restart)],
    )
}

pub(crate) fn progress_view(state: &FlashingFailState) -> Element<'_, BBImagerMessage> {
    widget::column![
        CircleBar::new("Failed", 10.0, constants::DANGER),
        widget::text(&state.err)
    ]
    .align_x(iced::Center)
    .padding(16)
    .into()
}

pub(crate) fn info_view(state: &FlashingFailState) -> Element<'_, BBImagerMessage> {
    widget::column![
        widget::text("Logs").size(28).font(constants::FONT_BOLD),
        widget::rule::horizontal(2),
        selectable_text(&state.logs)
    ]
    .spacing(8)
    .into()
}
