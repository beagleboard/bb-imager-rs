use iced::{Element, widget};

use crate::{
    Message,
    constants::{self, FONT_BOLD},
};

const HEADING_SIZE: u32 = 26;

#[derive(Default)]
pub(crate) struct State<'a> {
    pub(crate) title: &'static str,
    pub(crate) subtitle: &'static str,
    /// Summary row, omitted when `None`.
    pub(crate) board: Option<&'a str>,
    /// Summary row, omitted when `None`.
    pub(crate) image: Option<&'a str>,
    pub(crate) destination: &'a str,
    pub(crate) modifications_title: &'static str,
    pub(crate) modifications: &'a [&'static str],
    pub(crate) footer: Option<&'static str>,
}

impl<'a> State<'a> {
    pub(crate) fn view(&self, scroll_id: widget::Id) -> Element<'a, Message> {
        let summary = [
            ("Device", self.board),
            ("Operating System", self.image),
            ("Storage", Some(self.destination)),
        ]
        .into_iter()
        .filter_map(|(k, v)| v.map(|v| (k, v)))
        .flat_map(|(k, v)| [k.into(), v.into()]);

        let mut col = widget::column![
            widget::text(self.title)
                .font(constants::FONT_BOLD)
                .size(HEADING_SIZE),
            widget::text(self.subtitle).style(widget::text::primary),
            widget::rule::horizontal(2),
            widget::text("Summary")
                .font(constants::FONT_BOLD)
                .size(HEADING_SIZE),
            widget::grid(summary)
                .height(iced::Length::Shrink)
                .spacing(8)
                .columns(2),
        ];

        if !self.modifications.is_empty() {
            col = col.extend([
                widget::rule::horizontal(2).into(),
                widget::text(self.modifications_title)
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
            col = col.push(
                widget::container(widget::text(f).font(FONT_BOLD))
                    .style(widget::container::primary)
                    .padding(4),
            )
        }

        crate::helpers::detail_pane(col, &scroll_id)
    }
}
