use iced::{
    widget::{self, text},
    Element,
};

use crate::{
    constants,
    helpers::{self, home_btn_svg, home_btn_text},
    BBImagerMessage,
};

use super::{image_selection::ImageSelectionPage, Screen};

pub fn view<'a>(
    selected_board: Option<&'a bb_imager::config::Device>,
    selected_image: Option<&'a helpers::BoardImage>,
    selected_dst: Option<&'a bb_imager::Destination>,
    destination_selectable: bool,
) -> Element<'a, BBImagerMessage> {
    widget::responsive(move |size| {
        let choose_device_btn = match selected_board {
            Some(x) => home_btn_text(&x.name, true, iced::Length::Fill),
            None => home_btn_text("CHOOSE DEVICE", true, iced::Length::Fill),
        }
        .width(iced::Length::Fill)
        .on_press(BBImagerMessage::PushScreen(Screen::BoardSelection));

        let choose_image_btn = match selected_image {
            Some(x) => home_btn_text(x.to_string(), true, iced::Length::Fill),
            None => home_btn_text("CHOOSE IMAGE", selected_board.is_some(), iced::Length::Fill),
        }
        .width(iced::Length::Fill)
        .on_press_maybe(selected_board.map(|board| {
            BBImagerMessage::PushScreen(Screen::ImageSelection(ImageSelectionPage::new(
                board.flasher,
            )))
        }));

        let choose_dst_btn = match selected_dst {
            Some(x) => home_btn_text(x.to_string(), destination_selectable, iced::Length::Fill),
            None => home_btn_text(
                "CHOOSE DESTINATION",
                selected_image.is_some() && destination_selectable,
                iced::Length::Fill,
            ),
        }
        .width(iced::Length::Fill)
        .on_press_maybe(if selected_image.is_none() || !destination_selectable {
            None
        } else {
            Some(BBImagerMessage::PushScreen(Screen::DestinationSelection))
        });

        let reset_btn = home_btn_text("RESET", true, iced::Length::Fill)
            .on_press(BBImagerMessage::Reset)
            .width(iced::Length::Fill);

        let config_btn_active = selected_board.is_some() && selected_image.is_some();
        let config_btn = home_btn_svg(constants::SETTINGS_ICON, config_btn_active).on_press_maybe(
            if config_btn_active {
                Some(BBImagerMessage::PushScreen(Screen::ExtraConfiguration))
            } else {
                None
            },
        );

        let next_btn_active =
            selected_board.is_none() || selected_image.is_none() || selected_dst.is_none();
        let next_btn = home_btn_text("WRITE", !next_btn_active, iced::Length::Fill)
            .width(iced::Length::Fill)
            .on_press_maybe(if next_btn_active {
                None
            } else {
                Some(BBImagerMessage::PushScreen(Screen::FlashingConfirmation))
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
        .spacing(size.width * 0.03)
        .width(iced::Length::Fill)
        .align_y(iced::Alignment::Center);

        let action_btn_row = widget::row![
            reset_btn,
            widget::horizontal_space(),
            config_btn,
            widget::horizontal_space(),
            next_btn
        ]
        .width(iced::Length::Fill)
        .align_y(iced::Alignment::Center);

        let bottom = widget::center(
            widget::column![
                choice_btn_row.height(iced::Length::FillPortion(1)),
                action_btn_row.height(iced::Length::FillPortion(1)),
            ]
            .height(iced::Length::Fill)
            .align_x(iced::Alignment::Center),
        )
        .padding([0.0, size.width * 0.05])
        .style(|_| widget::container::background(constants::BEAGLE_BRAND_COLOR));

        widget::column![
            widget::container(
                widget::image(widget::image::Handle::from_bytes(constants::BB_BANNER))
                    .width(size.width * 0.45)
                    .height(size.height / 4.0),
            )
            .padding(iced::Padding::new(0.0).left(40))
            .width(iced::Length::Fill),
            bottom
        ]
        .into()
    })
    .into()
}
