use iced::{Element, widget};

use crate::helpers::{SidebarEntry, VIEW_COL_PADDING, page_type2};
use crate::{Message, constants};

#[derive(Debug)]
pub struct State {
    pub reason: Box<str>,
    /// Per-destination outcome, as one line each, when several destinations were written.
    pub details: Box<[Box<str>]>,
    pub logs: widget::text_editor::Content,
    /// Whether this run has a Modify step.
    pub has_customization: bool,
}

pub fn view(state: &State) -> Element<'_, Message> {
    page_type2(
        SidebarEntry::Write,
        state.has_customization,
        false,
        fail_info_view(
            format!("Write Failed: {}", state.reason),
            "Writing to the device faild. The device might be in an unusable state at present.",
            &state.details,
            &state.logs,
        ),
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

/// Failure reason followed by the run's logs. Shared with
/// [`crate::format_media_fail`].
pub(crate) fn fail_info_view<'a>(
    heading: String,
    desc: &'static str,
    details: &'a [Box<str>],
    logs: &'a widget::text_editor::Content,
) -> Element<'a, Message> {
    widget::column![
        widget::text(heading).size(28).font(constants::FONT_BOLD),
        widget::text(desc).style(widget::text::danger),
        widget::column(details.iter().map(|x| {
            widget::rich_text![
                widget::span::<'_, (), _>("• "),
                widget::span::<'_, (), _>(x.as_ref())
            ]
            .into()
        }))
        .spacing(4),
        widget::rule::horizontal(2),
        widget::text("Logs").size(26).font(constants::FONT_BOLD),
        widget::text("Here are logs for the current run. These can be used for debugging."),
        widget::container(widget::text_editor(logs).on_action(Message::EditorEvent))
            .padding(iced::Padding::ZERO.right(16))
    ]
    .spacing(8)
    .padding(VIEW_COL_PADDING)
    .into()
}
