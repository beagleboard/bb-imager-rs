use iced::{
    Element,
    widget::{self, button, column, row, text},
};

use crate::{
    BBImagerMessage,
    ui::helpers::{self, svg_icon_style},
};
use crate::{
    constants,
    ui::helpers::{card_btn_style, page_type1},
};

const ICON_WIDTH: u32 = 100;

pub(crate) fn view<'a>(state: &'a crate::ChooseBoardState) -> Element<'a, BBImagerMessage> {
    page_type1(
        &state.common,
        board_list_pane(state),
        board_view_pane(state),
        [widget::button("NEXT")
            .on_press_maybe(state.selected_board.map(|_| BBImagerMessage::Next))],
    )
}

fn board_list_pane<'a>(state: &'a crate::ChooseBoardState) -> Element<'a, BBImagerMessage> {
    let items = state
        .devices()
        .map(|(id, dev)| {
            let is_selected = state.selected_board.map(|x| x == id).unwrap_or(false);
            let img: Element<BBImagerMessage> = match &dev.icon {
                Some(u) => match state.image_handle_cache().get(u) {
                    Some(handle) => handle.view(ICON_WIDTH, iced::Shrink),
                    _ => widget::svg(state.downloading_svg().clone())
                        .width(ICON_WIDTH)
                        .style(svg_icon_style)
                        .into(),
                },
                None => widget::svg(state.board_svg().clone())
                    .width(ICON_WIDTH)
                    .style(svg_icon_style)
                    .into(),
            };
            button(
                row![img, text(&dev.name).size(18).width(iced::Length::Fill)]
                    .spacing(12)
                    .padding(8)
                    .align_y(iced::alignment::Vertical::Center),
            )
            .on_press(BBImagerMessage::SelectBoard(id))
            .style(move |theme, status| card_btn_style(theme, status, is_selected))
        })
        .map(Into::into);

    widget::scrollable(column(items).padding(iced::Padding::ZERO.right(12))).into()
}

fn board_view_pane<'a>(state: &'a crate::ChooseBoardState) -> Element<'a, BBImagerMessage> {
    match state.selected_board() {
        Some(dev) => helpers::board_view_pane(dev, &state.common),
        None => widget::center(
            text("Please Select a Board")
                .font(constants::FONT_BOLD)
                .size(28),
        )
        .into(),
    }
}
