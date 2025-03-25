use iced::{
    Element,
    widget::{self, button, text},
};

use crate::{BBImagerMessage, constants, helpers};

pub(crate) fn view<'a, D>(destinations: D, search_bar: &'a str) -> Element<'a, BBImagerMessage>
where
    D: Iterator<Item = &'a helpers::Destination>,
{
    let mut sorted_destinations = Vec::from_iter(destinations);
    sorted_destinations.sort_by(|a, b| a.size().cmp(&b.size()));
    let items = sorted_destinations
        .into_iter()
        .filter(|x| {
            x.to_string()
                .to_lowercase()
                .contains(&search_bar.to_lowercase())
        })
        .map(|x| {
            let mut row2 = widget::column![text(x.to_string())];

            if let Some(size) = x.size() {
                row2 = row2.push(text(helpers::format_size(size)));
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
        helpers::search_bar(search_bar),
        widget::horizontal_rule(2),
        row3
    ]
    .spacing(10)
    .padding(10)
    .into()
}
