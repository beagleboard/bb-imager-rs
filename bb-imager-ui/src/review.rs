use iced::widget;

use crate::Message;
use crate::constants::{COPY_ICON, FONT_BOLD};
use crate::helpers::{page_layout, pretty_bytes};

const HEADING_SIZE: u32 = 26;

#[derive(Default, Debug)]
pub struct State {
    pub has_customization: bool,
    pub device_name: Box<str>,
    pub software_name: Box<str>,
    pub storage: (Box<str>, Option<u64>),
    pub modificiations: Box<[&'static str]>,
}

pub fn view<'a, D: Clone + 'a>(
    s: &'a State,
    scroll_id: widget::Id,
) -> iced::Element<'a, Message<D>> {
    let mut sidebar = vec![
        ("Device", false, Some(Message::GotoDevicePage)),
        ("Software", false, Some(Message::GotoSoftwarePage)),
        ("Destination", false, Some(Message::GotoDestinationPage)),
    ];

    if s.has_customization {
        sidebar.push(("Customization", false, Some(Message::GotoCustomizationPage)));
    }

    sidebar.push(("Review", true, None));

    let storage_name: iced::Element<'a, _> = if let Some(size) = s.storage.1 {
        widget::rich_text([
            widget::span::<'_, (), _>(s.storage.0.as_ref()),
            widget::span(" ("),
            widget::span(pretty_bytes(size)),
            widget::span(")"),
        ])
        .into()
    } else {
        widget::text(s.storage.0.as_ref()).into()
    };

    let mut content = widget::column![
        widget::text("Write Image")
            .font(FONT_BOLD)
            .size(HEADING_SIZE),
        widget::text("Review your choices before flashing")
            .style(widget::text::primary)
            .font(FONT_BOLD),
        widget::rule::horizontal(2),
        widget::text("Summary").font(FONT_BOLD).size(HEADING_SIZE),
        widget::grid![
            widget::text("Device").font(FONT_BOLD),
            widget::text(s.device_name.as_ref()),
            widget::text("Software").font(FONT_BOLD),
            widget::text(s.software_name.as_ref()),
            widget::text("Storage").font(FONT_BOLD),
            storage_name
        ]
        .height(iced::Length::Shrink)
        .spacing(8)
        .columns(2),
    ];

    if !s.modificiations.is_empty() {
        content = content.extend([
            widget::rule::horizontal(2).into(),
            widget::text("Modifications to Apply")
                .size(HEADING_SIZE)
                .font(FONT_BOLD)
                .into(),
            widget::column(s.modificiations.iter().map(|x| {
                widget::rich_text![
                    widget::span::<'_, (), _>("• "),
                    widget::span::<'_, (), _>(*x)
                ]
                .into()
            }))
            .spacing(8)
            .into(),
        ]);
    }

    page_layout(
        (
            sidebar,
            [("App Options", false, Some(Message::GotoAppOptions))],
        ),
        widget::column![
            widget::scrollable(
                content
                    .spacing(16)
                    .padding(iced::Padding::ZERO.horizontal(16))
            )
            .id(scroll_id)
            .height(iced::Fill),
            widget::rule::horizontal(2),
            widget::row![
                widget::button(widget::svg(COPY_ICON.clone()).width(iced::Shrink))
                    .on_press(Message::CopyToClipboard),
                widget::space::horizontal(),
                widget::button("WRITE").on_press(Message::FlashStart)
            ]
            .align_y(iced::Center)
            .padding(iced::Padding::ZERO.horizontal(16))
        ]
        .padding(iced::Padding::ZERO.vertical(16))
        .spacing(16),
    )
}
