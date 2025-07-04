use iced::{
    Element,
    widget::{self, button, text},
};

use super::helpers::search_bar;
use crate::{BBImagerMessage, constants, helpers};

pub(crate) fn view<'a, D>(destinations: D, search_str: &'a str) -> Element<'a, BBImagerMessage>
where
    D: Iterator<Item = &'a helpers::Destination>,
{
    let items = destinations
        .into_iter()
        .filter(|x| {
            x.to_string()
                .to_lowercase()
                .contains(&search_str.to_lowercase())
        })
        .map(|x| {
            let mut row2 = widget::column![text(x.to_string())];

            if let Some(size) = x.size() {
                row2 = row2.push(text(format_size(size)));
            }

            button(
                widget::row![
                    widget::svg(widget::svg::Handle::from_memory(constants::USB_ICON)).width(40),
                    row2
                ]
                .align_y(iced::Alignment::Center)
                .spacing(10),
            )
            .width(iced::Length::Fill)
            .on_press(BBImagerMessage::SelectPort(x.clone()))
            .style(widget::button::secondary)
        })
        .map(Into::into);

    let row3: iced::Element<_> = if items.size_hint() == (0, Some(0)) {
        text("No destinations found")
            .width(iced::Length::Fill)
            .size(20)
            .color([0.8, 0.8, 0.8])
            .into()
    } else {
        widget::scrollable(widget::column(items).spacing(10)).into()
    };

    widget::column![
        search_bar(search_str, |x| BBImagerMessage::ReplaceScreen(
            crate::pages::Screen::DestinationSelection(crate::pages::SearchState::new(x))
        )),
        widget::horizontal_rule(2),
        row3
    ]
    .spacing(10)
    .padding(10)
    .into()
}

pub(crate) fn format_size(size: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * KB;
    const GB: f64 = 1024.0 * MB;
    const TB: f64 = 1024.0 * GB;

    if size < KB as u64 {
        format!("{size} B")
    } else if size < MB as u64 {
        format!("{:.2} KB", size as f64 / KB)
    } else if size < GB as u64 {
        format!("{:.2} MB", size as f64 / MB)
    } else if size < TB as u64 {
        format!("{:.2} GB", size as f64 / GB)
    } else {
        format!("{:.2} TB", size as f64 / TB)
    }
}
