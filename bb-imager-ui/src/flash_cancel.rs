use iced::widget;

use crate::Message;
use crate::constants::FONT_BOLD;
use crate::helpers::page_layout;

const HEADING_SIZE: u32 = 26;

#[derive(Default)]
pub struct State {
    pub has_customization: bool,
}

pub fn view<'a, D: Clone + 'a>(s: &'a State) -> iced::Element<'a, Message<D>> {
    let mut sidebar = vec![
        ("Device", false, Some(Message::GotoDevicePage)),
        ("Software", false, Some(Message::GotoSoftwarePage)),
        ("Destination", false, Some(Message::GotoDestinationPage)),
    ];

    if s.has_customization {
        sidebar.push(("Customization", false, Some(Message::GotoCustomizationPage)));
    }

    sidebar.push(("Review", false, Some(Message::GotoReviewPage)));
    sidebar.push(("Flashing", true, None));

    page_layout(
        (
            sidebar,
            [("App Options", false, Some(Message::GotoAppOptions))],
        ),
        widget::column![
            widget::column![
                widget::text("Write Cancelled")
                    .style(widget::text::danger)
                    .font(FONT_BOLD)
                    .size(HEADING_SIZE),
                widget::text("Writing cancelled by the user")
                    .style(widget::text::danger)
                    .font(FONT_BOLD),
            ]
            .height(iced::Fill)
            .spacing(16)
            .padding(iced::Padding::ZERO.horizontal(16)),
            widget::rule::horizontal(2),
            widget::right(widget::button("WRITE ANOTHER").on_press(Message::GotoDevicePage))
                .padding(iced::Padding::ZERO.horizontal(16))
        ]
        .height(iced::Fill)
        .padding(iced::Padding::ZERO.vertical(16))
        .spacing(16),
    )
}
