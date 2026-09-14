use iced::widget;

use crate::Message;
use crate::constants::{COPY_ICON, FONT_BOLD};
use crate::helpers::page_layout;

const HEADING_SIZE: u32 = 26;

#[derive(Default)]
pub struct State {
    pub has_customization: bool,
    pub reason: Box<str>,
    pub logs: widget::text_editor::Content,
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
                widget::text("Write Fail")
                    .style(widget::text::danger)
                    .font(FONT_BOLD)
                    .size(HEADING_SIZE),
                widget::text(s.reason.as_ref())
                    .style(widget::text::danger)
                    .font(FONT_BOLD),
                "Logs",
                widget::text_editor(&s.logs).on_action(Message::EditorEvent),
            ]
            .padding(iced::Padding::ZERO.horizontal(16))
            .spacing(16)
            .height(iced::Fill),
            widget::rule::horizontal(2),
            widget::row![
                widget::button(widget::svg(COPY_ICON.clone()).width(iced::Shrink))
                    .on_press(Message::CopyToClipboard),
                widget::space::horizontal(),
                widget::button("RETRY")
                    .style(widget::button::danger)
                    .on_press(Message::Retry),
                widget::button("WRITE ANOTHER").on_press(Message::GotoDevicePage)
            ]
            .padding(iced::Padding::ZERO.horizontal(16))
            .spacing(16)
        ]
        .height(iced::Fill)
        .padding(iced::Padding::ZERO.vertical(16))
        .spacing(16),
    )
}
