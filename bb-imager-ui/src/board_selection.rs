use std::sync::Arc;

use iced::{Element, widget};

use crate::helpers::{
    copy_btn, detail_entry, detail_pane, list_item, list_label, list_pane,
    network_image_or_default, page_type1, placeholder_pane,
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
    let Some(dev) = state.selected.as_ref() else {
        return placeholder_pane("Please Select a Board");
    };

    let img = network_image_or_default(
        cache,
        dev.icon.as_ref(),
        constants::BOARD_ICON.clone(),
        iced::Fill,
        iced::Shrink,
    );

    let copy_btn =
        copy_btn(constants::COPY_ICON.clone()).on_press(Message::CopyBoardConfig(dev.id));

    let cols = widget::column![
        img,
        widget::center(copy_btn),
        widget::text(dev.name.as_ref())
            .size(24)
            .align_x(iced::alignment::Alignment::Center)
            .width(iced::Length::Fill),
        widget::text(dev.description.as_ref())
            .align_x(iced::alignment::Alignment::Center)
            .width(iced::Length::Fill),
    ];

    let cols = cols.extend(
        dev.specification
            .iter()
            .map(|(k, v)| -> widget::text::Rich<'a, (), Message> { detail_entry(k, v.as_ref()) })
            .map(Into::into),
    );

    let mut btns = Vec::with_capacity(2);

    if let Some(x) = &dev.documentation {
        btns.push(
            widget::button(widget::text("DOCUMENTATION"))
                .on_press(Message::OpenUrl(x.clone()))
                .into(),
        );
    }

    if let Some(x) = &dev.oshw {
        btns.push(
            widget::button(widget::text("OSHW"))
                .on_press(Message::OpenUrl(x.clone()))
                .into(),
        );
    }

    detail_pane(
        cols.push(widget::center(widget::row(btns).spacing(16))),
        scroll_id,
    )
}
