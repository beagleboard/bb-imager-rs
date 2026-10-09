use iced::{Element, widget};

use crate::Message;
use crate::helpers::{SidebarEntry, page_type2};

#[derive(Debug)]
pub struct State {
    pub destination: Box<str>,
}

impl<'a> From<&'a State> for crate::review_inner::State<'a> {
    fn from(value: &'a State) -> Self {
        Self {
            title: "Format Media",
            subtitle: "Review your choices before formatting",
            destination: &value.destination,
            footer: Some("All data on this storage device will be permanently erased."),
            ..Default::default()
        }
    }
}

pub fn view<'a>(state: &'a State, scroll_id: widget::Id) -> Element<'a, Message> {
    page_type2(
        SidebarEntry::FormatMedia,
        false,
        false,
        crate::review_inner::State::from(state).view(scroll_id),
        [widget::button("FORMAT")
            .style(widget::button::danger)
            .on_press(Message::FlashStart)],
    )
}
