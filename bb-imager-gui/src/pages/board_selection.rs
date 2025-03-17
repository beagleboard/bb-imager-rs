use iced::{
    Element,
    widget::{self, button, text},
};

use crate::{
    BBImagerMessage, constants,
    helpers::{self, img_or_svg},
};

pub(crate) fn view<'a>(
    boards: &'a helpers::Boards,
    search_bar: &'a str,
    downloader: &'a bb_downloader::Downloader,
) -> Element<'a, BBImagerMessage> {
    let items = boards
        .devices()
        .filter(|(_, x)| x.name.to_lowercase().contains(&search_bar.to_lowercase()))
        .map(|(id, dev)| {
            let image: Element<BBImagerMessage> = match &dev.icon {
                Some(url) => match downloader.clone().check_cache_from_url(url) {
                    Some(y) => img_or_svg(y, 100),
                    None => widget::svg(widget::svg::Handle::from_memory(
                        constants::DOWNLOADING_ICON,
                    ))
                    .width(100)
                    .into(),
                },
                None => widget::svg(widget::svg::Handle::from_memory(constants::BOARD_ICON))
                    .width(100)
                    .height(60)
                    .into(),
            };

            button(
                widget::row![
                    image,
                    widget::column![
                        text(&dev.name).size(18),
                        widget::horizontal_space(),
                        text(dev.description.as_str())
                    ]
                    .padding(5)
                ]
                .align_y(iced::Alignment::Center)
                .spacing(10),
            )
            .width(iced::Length::Fill)
            .on_press(BBImagerMessage::BoardSelected(id))
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
