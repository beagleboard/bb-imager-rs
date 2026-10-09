use iced::{Element, widget};

use crate::Message;
use crate::helpers::{SidebarEntry, page_type2};

#[derive(Debug)]
pub struct State {
    pub board: Box<str>,
    pub image: Box<str>,
    pub destination: Box<str>,
    pub modifications: Box<[&'static str]>,
    /// Whether this run has a Modify step.
    pub has_customization: bool,
}

impl<'a> From<&'a State> for crate::review_inner::State<'a> {
    fn from(value: &'a State) -> Self {
        Self {
            title: "Write Complete",
            subtitle: "Device is ready to be used with your BeagleBoard hardware!",
            board: Some(&value.board),
            image: Some(&value.image),
            destination: &value.destination,
            modifications_title: "Modifications applied",
            modifications: &value.modifications,
            footer: Some(
                "The storage device was ejected automatically, you can now remove it safely.",
            ),
        }
    }
}

pub fn view<'a>(state: &'a State, scroll_id: widget::Id) -> Element<'a, Message> {
    page_type2(
        SidebarEntry::Write,
        state.has_customization,
        false,
        crate::review_inner::State::from(state).view(scroll_id),
        [widget::button("Flash Another")
            .style(widget::button::primary)
            .on_press(Message::Restart)],
    )
}
