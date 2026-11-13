use std::sync::Arc;

use bb_iced_widgets::cached_icon::Cache;
use iced::{Element, widget};

use crate::helpers::{
    SidebarEntry, copy_btn, detail_entry, detail_pane, list_item, list_label, list_pane,
    network_image_or_default, page_type1, placeholder_pane,
};
use crate::{Message, constants};

const ICON_WIDTH: u32 = 100;

/// One row of the board list.
#[derive(Default, Debug, Clone)]
pub struct Board {
    pub id: i64,
    pub icon: Option<Arc<url::Url>>,
    pub name: Box<str>,
}

#[derive(Debug, Clone)]
pub struct BoardDetails {
    pub id: i64,
    pub name: Box<str>,
    pub icon: Option<Arc<url::Url>>,
    pub description: Arc<str>,
    pub specification: Box<[(Box<str>, Box<str>)]>,
    pub buttons: Box<[(&'static str, url::Url)]>,
    pub flasher: bb_config::config::Flasher,
    pub instructions: Option<Box<str>>,
}

#[derive(Default, Debug)]
pub struct State {
    pub boards: Box<[Board]>,
    pub selected: Option<BoardDetails>,
    pub search: Arc<str>,
}

pub fn view<'a>(
    cache: &'a Cache<Arc<url::Url>>,
    state: &'a State,
    scroll_id: widget::Id,
) -> Element<'a, Message> {
    page_type1(
        SidebarEntry::Hardware,
        board_list_pane(cache, state, &scroll_id),
        board_view_pane(cache, state, &scroll_id),
        [widget::button("NEXT").on_press_maybe(state.selected.as_ref().map(|_| Message::Next))],
    )
}

fn board_list_pane<'a>(
    cache: &'a Cache<Arc<url::Url>>,
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
    cache: &'a Cache<Arc<url::Url>>,
    state: &'a State,
    scroll_id: &widget::Id,
) -> Element<'a, Message> {
    match state.selected.as_ref() {
        Some(dev) => board_details_pane(cache, dev, scroll_id),
        None => placeholder_pane("Please Select a Board"),
    }
}

/// The pane detailing one board: icon, name, description, specification table
/// and its documentation/OSHW links.
fn board_details_pane<'a>(
    cache: &'a bb_iced_widgets::cached_icon::Cache<std::sync::Arc<url::Url>>,
    dev: &'a crate::board_selection::BoardDetails,
    scroll_id: &widget::Id,
) -> Element<'a, Message> {
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

    let btns = dev.buttons.iter().map(|(label, link)| {
        widget::button(widget::text(*label))
            .on_press(Message::OpenUrl(link.clone()))
            .into()
    });

    detail_pane(
        cols.push(widget::center(widget::row(btns).spacing(16))),
        scroll_id,
    )
}
