use iced::{
    Element,
    widget::{self, text},
};

use crate::{
    constants,
    message::BBImagerMessage,
    state::CustomizeState,
    ui::helpers::{VIEW_COL_PADDING, page_type2},
};

const HEADING_SIZE: u32 = 26;

pub(crate) fn view<'a>(state: &'a CustomizeState) -> Element<'a, BBImagerMessage> {
    let btn_label = if state.is_download() {
        "DOWNLOAD"
    } else {
        "WRITE"
    };

    page_type2(
        review_view(state),
        [
            widget::button("BACK")
                .on_press(BBImagerMessage::Back)
                .style(widget::button::secondary),
            widget::button(btn_label).on_press(BBImagerMessage::FlashStart),
        ],
    )
}

fn review_view<'a>(state: &'a CustomizeState) -> Element<'a, BBImagerMessage> {
    let mut col = widget::column![
        text("Write Image")
            .font(constants::FONT_BOLD)
            .size(HEADING_SIZE),
        text("Review your choices before flashing").style(widget::text::primary),
        widget::rule::horizontal(2),
        text("Summary")
            .font(constants::FONT_BOLD)
            .size(HEADING_SIZE),
        widget::grid![
            text("Device"),
            text(state.selected_board()),
            text("Operating System"),
            text(state.selected_image()),
            text("Storage"),
            text(state.selected_destination())
        ]
        .height(iced::Length::Shrink)
        .spacing(8)
        .columns(2),
    ];

    let modifications = state.modifications();
    if !modifications.is_empty() {
        col = col.extend([
            widget::rule::horizontal(2).into(),
            text("Modifications to apply")
                .font(constants::FONT_BOLD)
                .size(HEADING_SIZE)
                .into(),
            widget::column(state.modifications().into_iter().map(Into::into))
                .spacing(8)
                .into(),
        ]);
    }

    widget::scrollable(col.spacing(16).padding(VIEW_COL_PADDING))
        .id(state.common.scroll_id.clone())
        .into()
}
