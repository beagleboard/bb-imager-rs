use iced::{Element, advanced::text, widget};

use crate::constants;

use super::BBImagerMessage;

pub(crate) fn dialog<'a>(
    base: Element<'a, BBImagerMessage>,
    menu: Element<'a, BBImagerMessage>,
) -> Element<'a, BBImagerMessage> {
    let overlay = widget::opaque(widget::center(menu).style(|_| {
        widget::container::background(iced::Color {
            a: 0.8,
            ..iced::Color::BLACK
        })
    }));
    widget::stack![base, overlay].into()
}

pub(crate) fn search_bar<'a>(
    cur_search: &'a str,
    f: impl Fn(String) -> BBImagerMessage + 'a,
) -> Element<'a, BBImagerMessage> {
    widget::row![
        widget::button(
            widget::svg(widget::svg::Handle::from_memory(constants::ARROW_BACK_ICON)).width(22)
        )
        .on_press(BBImagerMessage::PopScreen)
        .style(widget::button::secondary),
        widget::text_input("Search", cur_search).on_input(f)
    ]
    .spacing(10)
    .into()
}

pub(crate) fn img_or_svg<'a>(path: std::path::PathBuf, width: u16) -> Element<'a, BBImagerMessage> {
    let img = std::fs::read(path).expect("Failed to open image");

    match image::guess_format(&img) {
        Ok(_) => widget::image(widget::image::Handle::from_bytes(img))
            .width(width)
            .height(width)
            .into(),

        Err(_) => widget::svg(widget::svg::Handle::from_memory(img))
            .width(width)
            .height(width)
            .into(),
    }
}

pub(crate) fn home_btn_text<'a>(
    txt: impl text::IntoFragment<'a>,
    active: bool,
    text_width: iced::Length,
) -> widget::Button<'a, BBImagerMessage> {
    fn style(active: bool) -> widget::button::Style {
        if active {
            widget::button::Style {
                background: Some(iced::Color::WHITE.into()),
                text_color: constants::BEAGLE_BRAND_COLOR,
                border: iced::border::rounded(4),
                ..Default::default()
            }
        } else {
            widget::button::Style {
                background: Some(iced::Color::BLACK.scale_alpha(0.5).into()),
                text_color: iced::Color::BLACK.scale_alpha(0.8),
                border: iced::border::rounded(4),
                ..Default::default()
            }
        }
    }

    widget::button(
        widget::text(txt)
            .font(constants::FONT_BOLD)
            .align_x(iced::Alignment::Center)
            .align_y(iced::Alignment::Center)
            .width(text_width),
    )
    .padding(8)
    .style(move |_, _| style(active))
}

pub(crate) fn home_btn_svg<'a>(
    icon: &'static [u8],
    active: bool,
) -> widget::Button<'a, BBImagerMessage> {
    const ICON_SIZE: u16 = 32;
    const PADDING: u16 = 4;
    const RADIUS: u16 = (ICON_SIZE + PADDING * 2) / 2;

    fn svg_style(active: bool) -> widget::svg::Style {
        if active {
            Default::default()
        } else {
            widget::svg::Style {
                color: Some(iced::Color::BLACK.scale_alpha(0.5)),
            }
        }
    }

    fn btn_style(active: bool) -> widget::button::Style {
        if active {
            widget::button::Style {
                background: Some(iced::Color::WHITE.into()),
                border: iced::border::rounded(RADIUS),
                ..Default::default()
            }
        } else {
            widget::button::Style {
                background: Some(iced::Color::BLACK.scale_alpha(0.5).into()),
                border: iced::border::rounded(RADIUS),
                ..Default::default()
            }
        }
    }

    widget::button(
        widget::svg(widget::svg::Handle::from_memory(icon))
            .style(move |_, _| svg_style(active))
            .width(ICON_SIZE)
            .height(ICON_SIZE),
    )
    .style(move |_, _| btn_style(active))
    .padding(PADDING)
}
