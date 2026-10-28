use iced::{Element, widget};

use crate::Message;
use crate::flash_fail::fail_info_view;
use crate::helpers::{SidebarEntry, page_type2};

#[derive(Debug)]
pub struct State {
    /// Each failed device's label paired with why it failed.
    pub failures: Box<[(Box<str>, Box<str>)]>,
    pub logs: widget::text_editor::Content,
}

pub fn view(state: &State, scroll_id: widget::Id) -> Element<'_, Message> {
    page_type2(
        SidebarEntry::FormatMedia,
        false,
        false,
        fail_info_view(
            "Format Failed".to_string(),
            "Formatting failed. The device might be in an unusable state at present.",
            &state.failures,
            &state.logs,
            scroll_id,
        ),
        [
            widget::button("Format Another")
                .style(widget::button::danger)
                .on_press(Message::Goto(SidebarEntry::FormatMedia)),
            widget::button("Retry")
                .style(widget::button::primary)
                .on_press(Message::Retry),
        ],
    )
}
