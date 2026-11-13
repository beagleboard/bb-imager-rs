use std::sync::Arc;

use iced::Element;
use iced::widget;

use crate::Message;
pub use crate::destination_selection::DestinationItem;
use crate::destination_selection::{device_list_pane, next_button};
use crate::helpers::{SidebarEntry, page_type2};

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
    let list = device_list_pane(
        &state.destinations,
        |id| state.selected.iter().any(|x| x.as_ref() == id),
        state.filter_destination,
        &state.search,
        &scroll_id,
        [],
    );

    page_type2(
        SidebarEntry::FormatMedia,
        false,
        false,
        list,
        [next_button(state.selected.len())],
    )
}
