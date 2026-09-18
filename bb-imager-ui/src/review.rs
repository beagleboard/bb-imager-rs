use iced::{Element, widget};

use crate::helpers::{detail_pane, page_type2};
use crate::{Message, constants};

const HEADING_SIZE: u32 = 26;

#[derive(Debug)]
pub struct State {
    pub is_download: bool,
    pub board: Box<str>,
    pub image: Box<str>,
    pub destination: Box<str>,
    pub modifications: Box<[&'static str]>,
}

pub fn view<'a>(state: &'a State, scroll_id: widget::Id) -> Element<'a, Message> {
    let btn_label = if state.is_download {
        "DOWNLOAD"
    } else {
        "WRITE"
    };

    page_type2(
        review_view(state, scroll_id),
        [
            widget::button("BACK")
                .on_press(Message::Back)
                .style(widget::button::secondary),
            widget::button(btn_label).on_press(Message::FlashStart),
        ],
    )
}

fn review_view<'a>(state: &'a State, scroll_id: widget::Id) -> Element<'a, Message> {
    let mut col = widget::column![
        widget::text("Write Image")
            .font(constants::FONT_BOLD)
            .size(HEADING_SIZE),
        widget::text("Review your choices before flashing").style(widget::text::primary),
        widget::rule::horizontal(2),
        widget::text("Summary")
            .font(constants::FONT_BOLD)
            .size(HEADING_SIZE),
        widget::grid![
            widget::text("Device"),
            widget::text(state.board.as_ref()),
            widget::text("Operating System"),
            widget::text(state.image.as_ref()),
            widget::text("Storage"),
            widget::text(state.destination.as_ref())
        ]
        .height(iced::Length::Shrink)
        .spacing(8)
        .columns(2),
    ];

    if !state.modifications.is_empty() {
        col = col.extend([
            widget::rule::horizontal(2).into(),
            widget::text("Modifications to apply")
                .font(constants::FONT_BOLD)
                .size(HEADING_SIZE)
                .into(),
            widget::column(state.modifications.iter().map(|x| {
                widget::rich_text![
                    widget::span::<'_, (), _>("• "),
                    widget::span::<'_, (), _>(*x)
                ]
                .into()
            }))
            .spacing(8)
            .into(),
        ]);
    }

    detail_pane(col, &scroll_id)
}
