use iced::{Element, widget};

use crate::{BBImagerMessage, state::ChooseBoardState, ui::helpers};

const ICON_WIDTH: u32 = 100;

pub(crate) fn view<'a>(state: &'a ChooseBoardState) -> Element<'a, BBImagerMessage> {
    helpers::page_type1(
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
            let is_selected = state
                .selected_board
                .as_ref()
                .map(|x| x.id == dev.id)
                .unwrap_or(false);
            let img = helpers::network_image_or_default(
                &state.common.img_handle_cache,
                dev.icon.as_ref(),
                helpers::BOARD_ICON.clone(),
                ICON_WIDTH,
                iced::Shrink,
            );
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
