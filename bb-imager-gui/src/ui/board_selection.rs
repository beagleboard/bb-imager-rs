use iced::{Element, widget};

use crate::{
    BBImagerMessage,
    state::ChooseBoardState,
    ui::helpers::{self, page_type1, svg_icon_style},
};

const ICON_WIDTH: u32 = 100;

pub(crate) fn view<'a>(state: &'a ChooseBoardState) -> Element<'a, BBImagerMessage> {
    page_type1(
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
                Some(u) => match state.common.img_handle_cache.get(u) {
                    Some(handle) => handle.view(ICON_WIDTH, iced::Shrink),
                    _ => iced_aw::Spinner::new().width(ICON_WIDTH).into(),
                },
                None => widget::svg(helpers::BOARD_ICON.clone())
                    .width(ICON_WIDTH)
                    .style(svg_icon_style)
                    .into(),
            };
            // TODO: Make selected_board proper id
            helpers::list_item(
                [img, helpers::list_label(&dev.name).into()],
                is_selected,
                BBImagerMessage::SelectBoardById(dev.id),
            )
        })
        .map(Into::into);

    helpers::list_pane(&state.search_text, &state.common.scroll_id, [], items)
}

fn board_view_pane<'a>(state: &'a ChooseBoardState) -> Element<'a, BBImagerMessage> {
    match state.selected_board.as_ref() {
        Some(dev) => helpers::board_view_pane(dev, &state.common),
        None => helpers::placeholder_pane("Please Select a Board"),
    }
}
