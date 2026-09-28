use iced::{Element, widget};

use crate::Message;
use crate::helpers::page_type2;

#[derive(Debug)]
pub struct State {
    pub board: Box<str>,
    pub image: Box<str>,
    pub destination: Box<str>,
    pub modifications: Box<[&'static str]>,
}

impl<'a> From<&'a State> for crate::review_inner::State<'a> {
    fn from(value: &'a State) -> Self {
        Self {
            title: "Write Complete",
            subtitle: "Device is ready to be used with your BeagleBoard hardware!",
            board: &value.board,
            image: &value.image,
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
        crate::review_inner::State::from(state).view(scroll_id),
        [widget::button("Flash Another")
            .style(widget::button::primary)
            .on_press(Message::Restart)],
    )
}
