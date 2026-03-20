use iced::{
    Element,
    widget::{self, button, column, row, text},
};

use crate::{
    BBImagerMessage,
    state::ChooseBoardState,
    ui::helpers::{self, LIST_COL_PADDING, VIEW_COL_PADDING, svg_icon_style},
};
use crate::{
    constants,
    ui::helpers::{card_btn_style, page_type1},
};

const ICON_WIDTH: u32 = 100;

pub(crate) fn view<'a>(state: &'a ChooseBoardState) -> Element<'a, BBImagerMessage> {
    page_type1(
        &state.common,
        board_list_pane(state),
        board_view_pane(state),
        [widget::button("NEXT")
            .on_press_maybe(state.selected_board.as_ref().map(|_| BBImagerMessage::Next))],
    )
}

fn board_list_pane<'a>(state: &'a ChooseBoardState) -> Element<'a, BBImagerMessage> {
    let items = state
        .boards
        .iter()
        .map(|dev| {
            // TODO: Make selected_board proper id
            let is_selected = state
                .selected_board
                .as_ref()
                .map(|x| x.id == dev.id)
                .unwrap_or(false);
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
            // TODO: Make selected_board proper id
            .on_press(BBImagerMessage::SelectBoardById(dev.id))
            .style(move |theme, status| card_btn_style(theme, status, is_selected))
        })
        .map(Into::into);

    widget::scrollable(column(items).padding(LIST_COL_PADDING))
        .id(state.common.scroll_id.clone())
        .into()
}

fn board_view_pane<'a>(state: &'a ChooseBoardState) -> Element<'a, BBImagerMessage> {
    match state.selected_board() {
        Some(dev) => helpers::board_view_pane(dev, &state.common),
        None => widget::center(
            text("Please Select a Board")
                .font(constants::FONT_BOLD)
                .size(28),
        )
        .padding(VIEW_COL_PADDING)
        .into(),
    }
}
