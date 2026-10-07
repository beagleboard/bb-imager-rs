use iced::{Element, widget};

use crate::Message;
use crate::helpers::{SidebarEntry, page_type2};

#[derive(Debug)]
pub struct State {
    pub is_download: bool,
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
            title: "Write Image",
            subtitle: "Review your choices before flashing",
            board: &value.board,
            image: &value.image,
            destination: &value.destination,
            modifications_title: "modifications to apply",
            modifications: &value.modifications,
            footer: None,
        }
    }
}

pub fn view<'a>(state: &'a State, scroll_id: widget::Id) -> Element<'a, Message> {
    let btn_label = if state.is_download {
        "DOWNLOAD"
    } else {
        "WRITE"
    };

    page_type2(
        SidebarEntry::Review,
        state.has_customization,
        false,
        crate::review_inner::State::from(state).view(scroll_id),
        [widget::button(btn_label).on_press(Message::FlashStart)],
    )
}
