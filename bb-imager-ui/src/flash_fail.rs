use iced::{Element, widget};

use crate::helpers::{VIEW_COL_PADDING, page_type1, progress_finish_view};
use crate::{Message, constants};

#[derive(Debug)]
pub struct State {
    pub reason: Box<str>,
    pub logs: widget::text_editor::Content,
}

pub fn view(state: &State) -> Element<'_, Message> {
    page_type1(
        info_view(state),
        progress_finish_view("Failed", constants::DANGER, state.reason.as_ref()),
        [
            widget::button("Flash New")
                .style(widget::button::danger)
                .on_press(Message::Restart),
            widget::button("Retry")
                .style(widget::button::primary)
                .on_press(Message::Retry),
        ],
    )
}

pub(crate) fn info_view(state: &State) -> Element<'_, Message> {
    widget::column![
        widget::text("Logs").size(28).font(constants::FONT_BOLD),
        widget::rule::horizontal(2),
        widget::container(widget::text_editor(&state.logs).on_action(Message::EditorEvent))
            .padding(iced::Padding::ZERO.right(16))
    ]
    .spacing(8)
    .padding(VIEW_COL_PADDING)
    .into()
}
