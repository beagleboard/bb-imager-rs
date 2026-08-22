use std::time::Instant;

use iced::widget;

use crate::Message;
use crate::constants::FONT_BOLD;
use crate::helpers::{page_layout, pretty_duration};

#[derive(Clone, Copy, Debug, Default)]
pub enum Progress {
    #[default]
    Preparing,
    Writing(f32),
    Verifying,
    Customizing,
}

#[derive(Debug, Default)]
pub struct State {
    pub has_customization: bool,
    pub progress: Progress,
    pub start_timestamp: Option<Instant>,
}

pub fn view<'a, D: Clone + 'a>(s: &'a State) -> iced::Element<'a, Message<D>> {
    let mut sidebar: Vec<(_, _, Option<Message<D>>)> = vec![
        ("Device", false, None),
        ("Software", false, None),
        ("Destination", false, None),
    ];

    if s.has_customization {
        sidebar.push(("Customization", false, None));
    }

    sidebar.extend([("Review", false, None), ("Flashing", true, None)]);

    let (progress_label, progress_bar) = match s.progress {
        Progress::Preparing => (
            widget::text("Preparing..."),
            widget::progress_bar(0.0..=1.0, 0.0),
        ),
        Progress::Writing(f) => (
            widget::text(format!("Writing... ({}%)", (f * 100.0) as u8)),
            widget::progress_bar(0.0..=1.0, f),
        ),
        Progress::Verifying => (
            widget::text("Verifying..."),
            widget::progress_bar(0.0..=1.0, 0.99),
        ),
        Progress::Customizing => (
            widget::text("Customizing..."),
            widget::progress_bar(0.0..=1.0, 0.99),
        ),
    };

    let time_remaining = match crate::helpers::time_remaining_from(
        s.progress,
        s.start_timestamp.map(|t| t.elapsed()),
    ) {
        Some(x) => widget::span::<'_, (), _>(pretty_duration(x)),
        None => widget::span("Calculating"),
    };

    page_layout(
        (
            sidebar,
            [("App Options", false, Some(Message::GotoAppOptions))],
        ),
        widget::column![
            widget::column![
                widget::text("Write Image").font(FONT_BOLD).size(26),
                widget::text("Do not disconnect the storage device!").style(widget::text::danger),
                progress_label
                    .font(FONT_BOLD)
                    .style(widget::text::secondary),
                progress_bar,
                widget::rich_text![
                    widget::span("Time Remaining: ").font(FONT_BOLD),
                    time_remaining
                ]
            ]
            .height(iced::Fill)
            .spacing(16)
            .padding(iced::Padding::ZERO.horizontal(16)),
            widget::rule::horizontal(2),
            widget::right(
                widget::button("CANCEL")
                    .style(widget::button::danger)
                    .on_press(Message::FlashCancel)
            )
            .padding(iced::Padding::ZERO.horizontal(16))
        ]
        .spacing(16)
        .padding(iced::Padding::ZERO.vertical(16)),
    )
}
