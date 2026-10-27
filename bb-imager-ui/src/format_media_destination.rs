use std::sync::Arc;

use iced::Element;
use iced::widget::{self, text};

use crate::helpers::{
    SidebarEntry, list_item, list_label, list_pane, list_separator, page_type2, svg_icon_style,
};
use crate::{Message, constants};

const ICON_WIDTH: u32 = 60;

/// One row of the device list.
#[derive(Debug, Clone)]
pub struct DestinationItem {
    /// The device's stable hardware identifier.
    pub id: Box<str>,
    pub label: Box<str>,
    /// Second line under the label — the device size, where there is one.
    pub subtitle: Option<Box<str>>,
}

#[derive(Debug)]
pub struct State {
    pub destinations: Box<[DestinationItem]>,
    /// Identifiers of the selected devices.
    pub selected: Vec<Box<str>>,
    pub filter_destination: bool,
    /// Only here so the search box can show what was typed; the host does the
    /// filtering and hands back a new [`State::destinations`].
    pub search: Arc<str>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            destinations: Box::default(),
            selected: Vec::new(),
            // Hand-written rather than derived: destinations start filtered.
            filter_destination: true,
            search: "".into(),
        }
    }
}

pub fn view<'a>(state: &'a State, scroll_id: widget::Id) -> Element<'a, Message> {
    let label = match state.selected.len() {
        n @ 2.. => text(format!("NEXT ({n})")),
        _ => text("NEXT"),
    };

    let next_btn = widget::button(label).on_press_maybe(if state.selected.is_empty() {
        None
    } else {
        Some(Message::Next)
    });

    page_type2(
        SidebarEntry::FormatMedia,
        false,
        false,
        dest_list_pane(state, &scroll_id),
        [next_btn],
    )
}

fn dest_list_pane<'a>(state: &'a State, scroll_id: &widget::Id) -> Element<'a, Message> {
    let items = state
        .destinations
        .iter()
        .map(|dest| {
            let is_selected = state
                .selected
                .iter()
                .any(|x| x.as_ref() == dest.id.as_ref());

            let label: Element<'_, _> = match dest.subtitle.as_ref() {
                Some(x) => widget::column![text(dest.label.as_ref()).size(18), text(x.as_ref())]
                    .width(iced::Length::Fill)
                    .into(),
                None => list_label(dest.label.as_ref()).into(),
            };

            list_item(
                [sized_icon(constants::USB_ICON.clone()), label],
                is_selected,
                Message::SelectDest(dest.id.clone()),
            )
        })
        .map(Into::into);
    let filter_toggle = widget::container(
        widget::toggler(!state.filter_destination)
            .label("Show all destinations")
            .on_toggle(|x| Message::DestinationFilter(!x)),
    )
    .padding(16);

    list_pane(
        &state.search,
        scroll_id,
        [filter_toggle.into(), list_separator()],
        items,
    )
}

fn sized_icon<'a>(handle: widget::svg::Handle) -> Element<'a, Message> {
    widget::svg(handle)
        .height(ICON_WIDTH)
        .width(ICON_WIDTH)
        .style(svg_icon_style)
        .into()
}
