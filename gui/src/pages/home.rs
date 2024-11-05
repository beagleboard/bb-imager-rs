use iced::{
    widget::{self, text},
    Element,
};

use crate::{
    helpers::{self, home_btn},
    BBImagerMessage,
};

use super::Screen;

pub fn view<'a>(
    selected_board: Option<&'a str>,
    selected_image: Option<&'a bb_imager::common::SelectedImage>,
    selected_dst: Option<&'a bb_imager::Destination>,
) -> Element<'a, BBImagerMessage> {
    let choose_device_btn = match selected_board {
        Some(x) => home_btn(x, true, iced::Length::Fill),
        None => home_btn("CHOOSE DEVICE", true, iced::Length::Fill),
    }
    .width(iced::Length::Fill)
    .on_press(BBImagerMessage::SwitchScreen(Screen::BoardSelection));

    let choose_image_btn = match selected_image {
        Some(x) => home_btn(x.to_string(), true, iced::Length::Fill),
        None => home_btn("CHOOSE IMAGE", selected_board.is_some(), iced::Length::Fill),
    }
    .width(iced::Length::Fill)
    .on_press_maybe(if selected_board.is_none() {
        None
    } else {
        Some(BBImagerMessage::SwitchScreen(Screen::ImageSelection))
    });

    let choose_dst_btn = match selected_dst {
        Some(x) => home_btn(x.to_string(), true, iced::Length::Fill),
        None => home_btn(
            "CHOOSE DESTINATION",
            selected_image.is_some(),
            iced::Length::Fill,
        ),
    }
    .width(iced::Length::Fill)
    .on_press_maybe(if selected_image.is_none() {
        None
    } else {
        Some(BBImagerMessage::SwitchScreen(Screen::DestinationSelection))
    });

    let reset_btn = home_btn("RESET", true, iced::Length::Fill)
        .on_press(BBImagerMessage::Reset)
        .width(iced::Length::Fill);

    let next_btn_active =
        selected_board.is_none() || selected_image.is_none() || selected_dst.is_none();

    let next_btn = home_btn("NEXT", !next_btn_active, iced::Length::Fill)
        .width(iced::Length::Fill)
        .on_press_maybe(if next_btn_active {
            None
        } else {
            Some(BBImagerMessage::SwitchScreen(Screen::ExtraConfiguration))
        });

    let choice_btn_row = widget::row![
        widget::column![
            text("BeagleBoard").color(iced::Color::WHITE),
            choose_device_btn
        ]
        .spacing(8)
        .width(iced::Length::FillPortion(1))
        .align_x(iced::Alignment::Center),
        widget::column![text("Image").color(iced::Color::WHITE), choose_image_btn]
            .spacing(8)
            .width(iced::Length::FillPortion(1))
            .align_x(iced::Alignment::Center),
        widget::column![
            text("Destination").color(iced::Color::WHITE),
            choose_dst_btn
        ]
        .spacing(8)
        .width(iced::Length::FillPortion(1))
        .align_x(iced::Alignment::Center)
    ]
    .padding(48)
    .spacing(48)
    .width(iced::Length::Fill)
    .align_y(iced::Alignment::Center);

    let action_btn_row = widget::row![
        reset_btn.width(iced::Length::FillPortion(1)),
        widget::horizontal_space().width(iced::Length::FillPortion(5)),
        next_btn.width(iced::Length::FillPortion(1))
    ]
    .padding(48)
    .width(iced::Length::Fill)
    .align_y(iced::Alignment::Center);

    let bottom = widget::container(
        widget::column![
            choice_btn_row.height(iced::Length::FillPortion(1)),
            action_btn_row.height(iced::Length::FillPortion(1))
        ]
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .align_x(iced::Alignment::Center),
    )
    .style(|_| {
        widget::container::background(iced::Color::parse("#aa5137").expect("unexpected error"))
    });

    widget::column![helpers::logo(), bottom]
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .align_x(iced::Alignment::Center)
        .into()
}
