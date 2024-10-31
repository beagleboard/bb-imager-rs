use iced::{
    widget::{self, button, text},
    Element,
};

use crate::{constants, helpers::img_or_svg, BBImagerMessage};

pub fn view(bbimager: &crate::BBImager) -> Element<BBImagerMessage> {
    let board = bbimager.selected_board.as_ref().unwrap();
    let items = bbimager
        .boards
        .images(board)
        .filter(|x| {
            x.name
                .to_lowercase()
                .contains(&bbimager.search_bar.to_lowercase())
        })
        .map(|x| {
            let mut row3 =
                widget::row![text(x.release_date.to_string()), widget::horizontal_space()]
                    .spacing(4)
                    .width(iced::Length::Fill);

            row3 = x
                .tags
                .iter()
                .fold(row3, |acc, t| acc.push(iced_aw::Badge::new(text(t))));

            let icon = match bbimager.downloader.clone().check_image(&x.icon) {
                Some(y) => img_or_svg(y, 80),
                None => widget::svg(widget::svg::Handle::from_memory(
                    constants::DOWNLOADING_ICON,
                ))
                .into(),
            };

            button(
                widget::row![
                    icon,
                    widget::column![
                        text(x.name.as_str()).size(18),
                        text(x.description.as_str()),
                        row3
                    ]
                    .padding(5)
                ]
                .align_y(iced::Alignment::Center)
                .spacing(10),
            )
            .width(iced::Length::Fill)
            .on_press(BBImagerMessage::SelectImage(
                bb_imager::SelectedImage::from(x),
            ))
            .style(widget::button::secondary)
        })
        .chain(std::iter::once(
            button(
                widget::row![
                    widget::svg(widget::svg::Handle::from_memory(constants::FILE_ADD_ICON))
                        .width(100),
                    text("Use Custom Image").size(18),
                ]
                .spacing(10),
            )
            .width(iced::Length::Fill)
            .on_press(BBImagerMessage::SelectLocalImage)
            .style(widget::button::secondary),
        ))
        .map(Into::into);

    widget::column![
        bbimager.search_bar(None),
        widget::horizontal_rule(2),
        widget::scrollable(widget::column(items).spacing(10))
    ]
    .spacing(10)
    .padding(10)
    .into()
}
