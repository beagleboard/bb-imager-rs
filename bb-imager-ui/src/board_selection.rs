use std::sync::Arc;

use iced::{Element, widget};

use crate::helpers::{
    board_details_pane, list_item, list_label, list_pane, network_image_or_default, page_type1,
    placeholder_pane,
};
use crate::{Message, constants};

const ICON_WIDTH: u32 = 100;

/// One row of the board list.
#[derive(Default, Debug, Clone)]
pub struct Board {
    pub id: i64,
    /// `Arc` so that cloning into the icon cache is a refcount bump rather than
    /// a `Url` clone, and so the row stays small.
    pub icon: Option<Arc<url::Url>>,
    pub name: Box<str>,
}

/// The selected board, as the detail pane renders it.
#[derive(Debug, Clone)]
pub struct BoardDetails {
    pub id: i64,
    pub name: Box<str>,
    pub icon: Option<Arc<url::Url>>,
    pub description: Box<str>,
    pub specification: Box<[(Box<str>, Box<str>)]>,
    pub documentation: Option<url::Url>,
    /// Already resolved to a full URL by the host.
    pub oshw: Option<url::Url>,
}

#[derive(Default, Debug)]
pub struct State {
    pub boards: Box<[Board]>,
    pub selected: Option<BoardDetails>,
    /// Only here so the search box can show what was typed; the host does the
    /// filtering and hands back a new [`State::boards`].
    pub search: Arc<str>,
}

/// The icon cache is shared with the image selection page and filled
/// asynchronously by the host, so it is borrowed rather than owned by [`State`].
pub fn view<'a>(
    cache: &'a bb_iced_widgets::cached_icon::Cache<Arc<url::Url>>,
    state: &'a State,
    scroll_id: widget::Id,
) -> Element<'a, Message> {
    page_type1(
        board_list_pane(cache, state, &scroll_id),
        board_view_pane(cache, state, &scroll_id),
        [widget::button("NEXT").on_press_maybe(state.selected.as_ref().map(|_| Message::Next))],
    )
}

fn board_list_pane<'a>(
    cache: &'a bb_iced_widgets::cached_icon::Cache<Arc<url::Url>>,
    state: &'a State,
    scroll_id: &widget::Id,
) -> Element<'a, Message> {
    let items = state
        .boards
        .iter()
        .map(|dev| {
            let is_selected = state
                .selected
                .as_ref()
                .map(|x| x.id == dev.id)
                .unwrap_or(false);
            let img = network_image_or_default(
                cache,
                dev.icon.as_ref(),
                constants::BOARD_ICON.clone(),
                ICON_WIDTH,
                iced::Shrink,
            );
            list_item(
                [img, list_label(dev.name.as_ref()).into()],
                is_selected,
                Message::SelectBoardById(dev.id),
            )
        })
        .map(Into::into);

    list_pane(&state.search, scroll_id, [], items)
}

fn board_view_pane<'a>(
    cache: &'a bb_iced_widgets::cached_icon::Cache<Arc<url::Url>>,
    state: &'a State,
    scroll_id: &widget::Id,
) -> Element<'a, Message> {
    match state.selected.as_ref() {
        Some(dev) => board_details_pane(cache, dev, scroll_id),
        None => placeholder_pane("Please Select a Board"),
    }
}
