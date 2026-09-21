use std::sync::Arc;

use iced::Element;
use iced::widget::{self, text};

use crate::helpers::{
    detail_entry, detail_pane, list_item, list_label, list_pane, list_separator, page_type1,
    placeholder_heading, svg_icon_style,
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
    SaveToFile,
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

/// The selected destination, as the detail pane renders it.
#[derive(Debug, Clone)]
pub struct DestinationDetails {
    pub id: DestId,
    pub title: Box<str>,
    pub details: Box<[(Box<str>, Box<str>)]>,
}

#[derive(Debug)]
pub struct State {
    /// Enumerated devices only. The "Save To File" row is derived from
    /// [`State::image_file_name`] at render time, so no host update can drop it.
    pub destinations: Box<[DestinationItem]>,
    pub selected: Option<DestinationDetails>,
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
    /// Index of the row the cursor is over, for the hover outline. The derived
    /// "Save To File" row occupies index [`State::destinations`]`::len()`.
    pub hovered: Option<usize>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            destinations: Box::default(),
            selected: None,
            // Hand-written rather than derived: destinations start filtered.
            filter_destination: true,
            search: "".into(),
            image_file_name: None,
            instruction: None,
            hovered: None,
        }
    }
}

pub fn view<'a>(state: &'a State, scroll_id: widget::Id) -> Element<'a, Message> {
    page_type1(
        dest_list_pane(state, &scroll_id),
        dest_view_pane(state, &scroll_id),
        [
            widget::button("BACK")
                .on_press(Message::Back)
                .style(widget::button::secondary),
            widget::button("NEXT").on_press_maybe(state.selected.as_ref().map(|_| Message::Next)),
        ],
    )
}

fn dest_list_pane<'a>(state: &'a State, scroll_id: &widget::Id) -> Element<'a, Message> {
    let items = state
        .destinations
        .iter()
        .enumerate()
        .map(|(index, dest)| {
            let is_selected = match state.selected.as_ref().map(|x| &x.id) {
                Some(DestId::Device(sel)) => sel.as_ref() == dest.id.as_ref(),
                _ => false,
            };

            let label: Element<'_, _> = match dest.subtitle.as_ref() {
                Some(x) => widget::column![text(dest.label.as_ref()).size(18), text(x.as_ref())]
                    .width(iced::Length::Fill)
                    .into(),
                None => list_label(dest.label.as_ref()).into(),
            };

            list_item(
                [device_icon(), label],
                is_selected,
                index,
                state.hovered,
                Message::SelectDest(dest.id.clone()),
            )
        });

    // Derived here rather than pushed in by the host, so that a refresh which
    // leaves the device list unchanged cannot drop it.
    let save_index = state.destinations.len();
    let save_to_file: Vec<Element<Message>> = match state.image_file_name.as_ref() {
        Some(name) => {
            let is_selected = state
                .selected
                .as_ref()
                .map(|x| x.id == DestId::SaveToFile)
                .unwrap_or(false);

            vec![
                list_item(
                    [
                        sized_icon(constants::FILE_SAVE_ICON.clone()),
                        list_label("Save To File").into(),
                    ],
                    is_selected,
                    save_index,
                    state.hovered,
                    Message::SelectFileDest(name.clone()),
                ),
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

fn dest_view_pane<'a>(state: &'a State, scroll_id: &widget::Id) -> Element<'a, Message> {
    let Some(dest) = state.selected.as_ref() else {
        let col = widget::column![placeholder_heading("Please Select a Destination")];

        let col = match state.instruction.as_ref() {
            Some(x) => col.extend([
                widget::rule::horizontal(2).into(),
                text("Special instructions")
                    .size(16)
                    .font(constants::FONT_BOLD)
                    .into(),
                text(x.as_ref()).into(),
            ]),
            None => col,
        };

        return widget::center(detail_pane(col, scroll_id)).into();
    };

    let icon = match dest.id {
        DestId::SaveToFile => widget::svg(constants::FILE_SAVE_ICON.clone()),
        DestId::Device(_) => widget::svg(constants::USB_ICON.clone()),
    }
    .height(100)
    .width(iced::Fill)
    .style(svg_icon_style);

    let col = widget::column![
        icon,
        text(dest.title.as_ref())
            .size(24)
            .align_x(iced::alignment::Alignment::Center)
            .width(iced::Length::Fill),
    ];

    let col = col.extend(
        dest.details
            .iter()
            .map(|(k, v)| detail_entry(k, v.as_ref()))
            .map(Into::into),
    );

    detail_pane(col, scroll_id)
}
