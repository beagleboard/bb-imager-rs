use iced::{
    widget::{self, button, text},
    Element,
};

use crate::{
    constants,
    helpers::{self, img_or_svg},
    BBImagerMessage,
};

pub fn view<'a>(
    boards: &'a helpers::Boards,
    search_bar: &'a str,
    downloader: &'a bb_imager::download::Downloader,
) -> Element<'a, BBImagerMessage> {
    let items = boards
        .devices()
        .filter(|(name, _)| name.to_lowercase().contains(&search_bar.to_lowercase()))
        .map(|(name, dev)| {
            let image: Element<BBImagerMessage> = match downloader.clone().check_image(&dev.icon) {
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

    widget::column![
        helpers::search_bar(search_bar),
        widget::horizontal_rule(2),
        items
    ]
    .spacing(10)
    .padding(10)
    .into()
}
