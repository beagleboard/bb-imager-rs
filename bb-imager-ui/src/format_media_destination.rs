use std::sync::Arc;

use iced::Element;
use iced::widget::{self, text};

use crate::helpers::{
    SidebarEntry, list_item, list_label, list_pane, list_separator, page_type4, svg_icon_style,
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
    pub filter_destination: bool,
    /// Only here so the search box can show what was typed; the host does the
    /// filtering and hands back a new [`State::destinations`].
    pub search: Arc<str>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            destinations: Box::default(),
            // Hand-written rather than derived: destinations start filtered.
            filter_destination: true,
            search: "".into(),
        }
    }
}

pub fn view<'a>(state: &'a State, scroll_id: widget::Id) -> Element<'a, Message> {
    page_type4(
        SidebarEntry::FormatMedia,
        false,
        dest_list_pane(state, &scroll_id),
    )
}

fn dest_list_pane<'a>(state: &'a State, scroll_id: &widget::Id) -> Element<'a, Message> {
    let items = state
        .destinations
        .iter()
        .map(|dest| {
            let label: Element<'_, _> = match dest.subtitle.as_ref() {
                Some(x) => widget::column![text(dest.label.as_ref()).size(18), text(x.as_ref())]
                    .width(iced::Length::Fill)
                    .into(),
                None => list_label(dest.label.as_ref()).into(),
            };

            list_item(
                [sized_icon(constants::USB_ICON.clone()), label],
                false,
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
