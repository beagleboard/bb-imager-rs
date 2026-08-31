use iced::{
    Element,
    widget::{self, text},
};

use crate::{
    constants,
    message::BBImagerMessage,
    state::CustomizeState,
    ui::helpers::{detail_pane, page_type2},
};

const HEADING_SIZE: u32 = 26;

pub(crate) fn view<'a>(state: &'a CustomizeState) -> Element<'a, BBImagerMessage> {
    let btn_label = if state.ctx.is_download() {
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
            text(state.ctx.selected_board.name.as_ref()),
            text("Operating System"),
            text(state.ctx.selected_image.1.to_string()),
            text("Storage"),
            text(state.ctx.selected_destination())
        ]
        .height(iced::Length::Shrink)
        .spacing(8)
        .columns(2),
    ];

    let modifications = state.ctx.customization.modifications();
    if !modifications.is_empty() {
        col = col.extend([
            widget::rule::horizontal(2).into(),
            text("Modifications to apply")
                .font(constants::FONT_BOLD)
                .size(HEADING_SIZE)
                .into(),
            widget::column(modifications.iter().map(|x| {
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

    detail_pane(col, &state.common.scroll_id)
}
