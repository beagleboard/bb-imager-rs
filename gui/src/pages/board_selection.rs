use iced::{
    widget::{self, button, text},
    Element,
};

use crate::{constants, helpers::img_or_svg, BBImagerMessage};

pub fn view(bbimager: &crate::BBImager) -> Element<BBImagerMessage> {
    let items = bbimager
        .boards
        .devices()
        .filter(|(name, _)| {
            name.to_lowercase()
                .contains(&bbimager.search_bar.to_lowercase())
        })
        .map(|(name, dev)| {
            let image: Element<BBImagerMessage> =
                match bbimager.downloader.clone().check_image(&dev.icon) {
                    Some(y) => img_or_svg(y, 100),
                    None => widget::svg(widget::svg::Handle::from_memory(
                        constants::DOWNLOADING_ICON,
                    ))
                    .width(40)
                    .into(),
                };

            button(
                widget::row![
                    image,
                    widget::column![
                        text(name).size(18),
                        widget::horizontal_space(),
                        text(dev.description.as_str())
                    ]
                    .padding(5)
                ]
                .align_y(iced::Alignment::Center)
                .spacing(10),
            )
            .width(iced::Length::Fill)
            .on_press(BBImagerMessage::BoardSelected(name.to_string()))
            .style(widget::button::secondary)
        })
        .map(Into::into);

    let items = widget::scrollable(widget::column(items).spacing(10));

    widget::column![bbimager.search_bar(None), widget::horizontal_rule(2), items]
        .spacing(10)
        .padding(10)
        .into()
}
