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
            title: "Format Complete",
            subtitle: "The storage device is ready to use.",
            destination: &value.destination,
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
        [widget::button("Format Another")
            .style(widget::button::primary)
            .on_press(Message::Goto(SidebarEntry::FormatMedia))],
    )
}
