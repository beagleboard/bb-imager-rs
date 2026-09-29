use iced::{Element, widget};

use crate::constants::FONT_BOLD;
use crate::helpers::{VIEW_COL_PADDING, page_type1, page_type2, progress_finish_view};
use crate::{Message, constants};

#[derive(Debug)]
pub struct State {
    pub reason: Box<str>,
    pub logs: widget::text_editor::Content,
}

pub fn view(state: &State) -> Element<'_, Message> {
    page_type2(
        info_view(state),
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
        widget::text(format!("Write Failed: {}", state.reason))
            .size(28)
            .font(constants::FONT_BOLD),
        widget::text(
            "Writing to the device faild. The device might be in an unusable state at present."
        )
        .style(widget::text::danger),
        widget::rule::horizontal(2),
        widget::text("Logs").size(26).font(constants::FONT_BOLD),
        widget::text("Here are logs for the current run. These can be used for debugging."),
        widget::container(widget::text_editor(&state.logs).on_action(Message::EditorEvent))
            .padding(iced::Padding::ZERO.right(16))
    ]
    .spacing(8)
    .padding(VIEW_COL_PADDING)
    .into()
}
