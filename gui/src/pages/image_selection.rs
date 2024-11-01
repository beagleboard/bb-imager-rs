use iced::{
    widget::{self, button, text},
    Element,
};

use crate::{
    constants,
    helpers::{self, img_or_svg},
    BBImagerMessage,
};

const ICON_WIDTH: u16 = 80;

pub fn view<'a, I>(
    images: I,
    search_bar: &'a str,
    downloader: &'a bb_imager::download::Downloader,
    // Allow optional format entry
    format_entry: Option<(&'a str, &'static [u8], BBImagerMessage)>,
) -> Element<'a, BBImagerMessage>
where
    I: Iterator<Item = &'a helpers::Image>,
{
    let custom_image_btn = custom_btn(
        "Use Custom Image",
        constants::FILE_ADD_ICON,
        BBImagerMessage::SelectLocalImage,
    );

    let items = images
        .filter(|x| x.name.to_lowercase().contains(&search_bar.to_lowercase()))
        .map(|x| {
            let row3 = widget::row(
                [
                    text(x.release_date.to_string()).into(),
                    widget::horizontal_space().into(),
                ]
                .into_iter()
                .chain(x.tags.iter().map(|t| iced_aw::badge(t.as_str()).into())),
            )
            .align_y(iced::alignment::Vertical::Center)
            .spacing(5);

            let icon = match downloader.clone().check_image(&x.icon) {
                Some(y) => img_or_svg(y, ICON_WIDTH),
                None => widget::svg(widget::svg::Handle::from_memory(
                    constants::DOWNLOADING_ICON,
                ))
                .width(ICON_WIDTH)
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
        .chain(match format_entry {
            Some((label, icon, msg)) => vec![custom_image_btn, custom_btn(label, icon, msg)],
            None => vec![custom_image_btn],
        })
        .map(Into::into);

    widget::column![
        helpers::search_bar(None, search_bar),
        widget::horizontal_rule(2),
        widget::scrollable(widget::column(items).spacing(10))
    ]
    .spacing(10)
    .padding(10)
    .into()
}

fn custom_btn<'a>(
    label: &'a str,
    icon: &'static [u8],
    msg: BBImagerMessage,
) -> widget::Button<'a, BBImagerMessage> {
    button(
        widget::row![
            widget::svg(widget::svg::Handle::from_memory(icon)).width(ICON_WIDTH),
            widget::container(text(label).size(18)).padding(5),
        ]
        .spacing(10),
    )
    .width(iced::Length::Fill)
    .on_press(msg)
    .style(widget::button::secondary)
}
