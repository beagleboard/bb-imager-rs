use std::sync::Arc;

use iced::Element;
use iced::widget::{self, text};

use crate::helpers::{
    SidebarEntry, detail_pane, list_item, list_label, list_pane, list_separator, page_type1,
    page_type2, svg_icon_style,
};
use crate::{Message, constants};

const ICON_WIDTH: u32 = 60;

/// Identifies the current selection.
///
/// Devices are keyed by their stable hardware identifier rather than by
/// position: the host re-enumerates every second, so an index could resolve to
/// a different device than the one that was clicked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DestId {
    Device(Box<str>),
    /// The image was written to a file rather than a device.
    SaveToFile(Box<std::path::Path>),
}

/// One row of the device list.
///
/// The "Save To File" row is not one of these — it is derived from
/// [`State::image_file_name`], so the two cannot disagree.
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
    /// Enumerated devices only. The "Save To File" row is derived from
    /// [`State::image_file_name`] at render time, so no host update can drop it.
    pub destinations: Box<[DestinationItem]>,
    pub selected: Vec<DestId>,
    pub filter_destination: bool,
    /// Only here so the search box can show what was typed; the host does the
    /// filtering and hands back a new [`State::destinations`].
    pub search: Arc<str>,
    /// Suggested file name, already normalized by the host. `Some` exactly when
    /// the image has something to write out, which is what offers the
    /// "Save To File" row.
    pub image_file_name: Option<Arc<str>>,
    /// The image's note, else the board's; shown only while nothing is selected.
    pub instruction: Option<Box<str>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            destinations: Box::default(),
            selected: Vec::new(),
            // Hand-written rather than derived: destinations start filtered.
            filter_destination: true,
            search: "".into(),
            image_file_name: None,
            instruction: None,
        }
    }
}

pub fn view<'a>(state: &'a State, scroll_id: widget::Id) -> Element<'a, Message> {
    let next_btn = widget::button("NEXT").on_press_maybe(if state.selected.is_empty() {
        None
    } else {
        Some(Message::Next)
    });

    if let Some(instr) = state.instruction.as_ref() {
        page_type1(
            SidebarEntry::Storage,
            dest_list_pane(state, &scroll_id),
            instr_view_pane(instr, &scroll_id),
            [next_btn],
        )
    } else {
        page_type2(
            SidebarEntry::Storage,
            false,
            false,
            dest_list_pane(state, &scroll_id),
            [next_btn],
        )
    }
}

fn dest_list_pane<'a>(state: &'a State, scroll_id: &widget::Id) -> Element<'a, Message> {
    let items = state
        .destinations
        .iter()
        .map(|dest| {
            let is_selected = state.selected.iter().any(|x| {
                if let DestId::Device(ident) = x {
                    dest.id.as_ref() == ident.as_ref()
                } else {
                    false
                }
            });

            let label: Element<'_, _> = match dest.subtitle.as_ref() {
                Some(x) => widget::column![text(dest.label.as_ref()).size(18), text(x.as_ref())]
                    .width(iced::Length::Fill)
                    .into(),
                None => list_label(dest.label.as_ref()).into(),
            };

            list_item(
                [device_icon(), label],
                is_selected,
                Message::SelectDest(dest.id.clone()),
            )
        })
        .map(Into::into);

    // Derived here rather than pushed in by the host, so that a refresh which
    // leaves the device list unchanged cannot drop it.
    let save_to_file: Vec<Element<Message>> = match state.image_file_name.as_ref() {
        Some(name) => {
            let is_selected = state
                .selected
                .iter()
                .any(|x| matches!(x, DestId::SaveToFile(_)));

            vec![
                list_item(
                    [
                        sized_icon(constants::FILE_SAVE_ICON.clone()),
                        list_label("Save To File").into(),
                    ],
                    is_selected,
                    Message::SelectFileDest(name.clone()),
                )
                .into(),
            ]
        }
        None => Vec::new(),
    };

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
        items.chain(save_to_file),
    )
}

fn device_icon<'a>() -> Element<'a, Message> {
    sized_icon(constants::USB_ICON.clone())
}

fn sized_icon<'a>(handle: widget::svg::Handle) -> Element<'a, Message> {
    widget::svg(handle)
        .height(ICON_WIDTH)
        .width(ICON_WIDTH)
        .style(svg_icon_style)
        .into()
}

fn instr_view_pane<'a>(instr: &'a str, scroll_id: &widget::Id) -> Element<'a, Message> {
    let col = widget::column![
        text("Special instructions")
            .size(16)
            .font(constants::FONT_BOLD),
        text(instr),
    ];

    widget::center(detail_pane(col, scroll_id)).into()
}
