use iced::{Element, widget};

use crate::{Message, constants};

const HEADING_SIZE: u32 = 26;

pub(crate) struct State<'a> {
    pub(crate) title: &'static str,
    pub(crate) subtitle: &'static str,
    pub(crate) board: &'a str,
    pub(crate) image: &'a str,
    pub(crate) destination: &'a str,
    pub(crate) modifications: &'a [&'static str],
    pub(crate) footer: Option<&'a str>,
}

impl<'a> State<'a> {
    pub(crate) fn view(&self, scroll_id: widget::Id) -> Element<'a, Message> {
        let mut col = widget::column![
            widget::text(self.title)
                .font(constants::FONT_BOLD)
                .size(HEADING_SIZE),
            widget::text(self.subtitle).style(widget::text::primary),
            widget::rule::horizontal(2),
            widget::text("Summary")
                .font(constants::FONT_BOLD)
                .size(HEADING_SIZE),
            widget::grid![
                widget::text("Device"),
                widget::text(self.board),
                widget::text("Operating System"),
                widget::text(self.image),
                widget::text("Storage"),
                widget::text(self.destination)
            ]
            .height(iced::Length::Shrink)
            .spacing(8)
            .columns(2),
        ];

        if !self.modifications.is_empty() {
            col = col.extend([
                widget::rule::horizontal(2).into(),
                widget::text("Modifications to apply")
                    .font(constants::FONT_BOLD)
                    .size(HEADING_SIZE)
                    .into(),
                widget::column(self.modifications.iter().map(|x| {
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

        if let Some(f) = self.footer {
            col = col.push(widget::container(f).style(widget::container::primary))
        }

        crate::helpers::detail_pane(col, &scroll_id)
    }
}
