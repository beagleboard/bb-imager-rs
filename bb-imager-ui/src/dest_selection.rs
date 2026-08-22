use iced::widget;

use crate::Message;
use crate::constants::{FILE_SAVE_ICON, FONT_BOLD, USB_ICON};
use crate::helpers::{list_item, page_layout, pretty_bytes, svg};

pub trait Destination: std::fmt::Display {
    fn size(&self) -> Option<u64>;
}

#[derive(Debug)]
pub struct State<T> {
    pub destinations: Box<[T]>,
    pub search: std::sync::Arc<str>,
    pub image_file_name: Option<std::sync::Arc<str>>,
    pub filter_destination: bool,
    pub instructions: Box<str>,
}

impl<T> Default for State<T> {
    fn default() -> Self {
        Self {
            destinations: Box::new([]),
            search: "".into(),
            image_file_name: None,
            filter_destination: true,
            instructions: "".into(),
        }
    }
}

pub fn view<'a, T: Destination + Clone>(
    s: &'a State<T>,
    scroll_id: widget::Id,
) -> iced::Element<'a, Message<T>> {
    let list = s.destinations.iter().map(|x| {
        let mut rows = Vec::new();

        if let Some(s) = x.size() {
            rows.push(
                widget::rich_text![
                    widget::span("Size: ").font(FONT_BOLD),
                    widget::span::<'_, (), _>(pretty_bytes(s))
                ]
                .into(),
            )
        };

        list_item(svg(USB_ICON.clone()), x.to_string(), rows, None)
            .on_press_with(|| Message::SelectDestination(x.clone()))
            .into()
    });

    let mut columns = widget::column(list);

    if let Some(fname) = &s.image_file_name {
        columns = columns.push(
            list_item(
                svg(FILE_SAVE_ICON.clone()),
                "Save To File",
                Vec::new(),
                None,
            )
            .on_press_with(|| Message::SelectFileDest(fname.clone())),
        );
    }

    let destinations = widget::scrollable(
        widget::container(columns.spacing(18)).padding(iced::Padding::from(15).right(20)),
    )
    .id(scroll_id);

    let last_row: iced::Element<'_, _> = if s.instructions.is_empty() {
        destinations.into()
    } else {
        widget::row![
            destinations,
            widget::rule::vertical(2),
            widget::center(s.instructions.as_ref())
        ]
        .into()
    };

    page_layout(
        (
            [
                ("Device", false, Some(Message::GotoDevicePage)),
                ("Software", false, Some(Message::GotoSoftwarePage)),
                ("Destination", true, None),
            ],
            [("App Options", false, Some(Message::GotoAppOptions))],
        ),
        widget::column![
            crate::helpers::search_box(&s.search),
            widget::rule::horizontal(2),
            widget::container(
                widget::toggler(!s.filter_destination)
                    .label("Show all destinations")
                    .on_toggle(|x| Message::DestinationFilter(!x))
            )
            .padding(15),
            widget::rule::horizontal(2),
            last_row
        ]
        .width(iced::Fill),
    )
}
