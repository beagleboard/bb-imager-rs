use iced::{Element, widget};

use crate::helpers::{SidebarEntry, VIEW_COL_PADDING, page_type2};
use crate::{Message, constants};

#[derive(Debug)]
pub struct State {
    pub reason: Box<str>,
    pub logs: widget::text_editor::Content,
    /// Whether this run has a Modify step.
    pub has_customization: bool,
}

pub fn view(state: &State, scroll_id: widget::Id) -> Element<'_, Message> {
    page_type2(
        SidebarEntry::Write,
        state.has_customization,
        false,
        fail_info_view(
            format!("Write Failed: {}", state.reason),
            "Writing to the device faild. The device might be in an unusable state at present.",
            &[],
            &state.logs,
            scroll_id,
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

/// Failure reason, then each failed device with its own reason, followed by
/// the run's logs. Shared with [`crate::format_media_fail`].
///
/// `failures` pairs a device label with why it failed; when empty, no
/// per-device section is shown.
pub(crate) fn fail_info_view<'a>(
    heading: String,
    desc: &'static str,
    failures: &'a [(Box<str>, Box<str>)],
    logs: &'a widget::text_editor::Content,
    scroll_id: widget::Id,
) -> Element<'a, Message> {
    let mut col = widget::column![
        widget::text(heading).size(28).font(constants::FONT_BOLD),
        widget::text(desc).style(widget::text::danger),
        widget::rule::horizontal(2),
    ];

    if !failures.is_empty() {
        col = col
            .extend(failures.iter().map(|(dest, reason)| {
                widget::column![
                    widget::text(dest.as_ref()).font(constants::FONT_BOLD),
                    widget::text(reason.as_ref()).style(widget::text::danger),
                ]
                .into()
            }))
            .push(widget::rule::horizontal(2));
    }

    col = col
        .push(widget::text("Logs").size(26).font(constants::FONT_BOLD))
        .push(widget::text(
            "Here are logs for the current run. These can be used for debugging.",
        ))
        .push(
            widget::container(widget::text_editor(logs).on_action(Message::EditorEvent))
                .padding(iced::Padding::ZERO.right(16)),
        )
        .spacing(8)
        .padding(VIEW_COL_PADDING);

    widget::scrollable(col).id(scroll_id).into()
}
