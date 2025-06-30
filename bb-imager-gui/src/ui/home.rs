use iced::{
    Element,
    widget::{self, text},
};

use crate::{BBImagerMessage, constants, helpers, pages::ImageSelectionState};

use super::helpers::home_btn_text;
use crate::pages::Screen;

pub(crate) fn view<'a>(
    selected_board: Option<&'a bb_config::config::Device>,
    selected_image: Option<&'a helpers::BoardImage>,
    selected_dst: Option<&'a helpers::Destination>,
    destination_selectable: bool,
) -> Element<'a, BBImagerMessage> {
    widget::responsive(move |size| {
        let choose_device_btn = match selected_board {
            Some(x) => home_btn_text(&x.name, true, iced::Length::Fill),
            None => home_btn_text("CHOOSE DEVICE", true, iced::Length::Fill),
        }
        .width(iced::Length::Fill)
        .on_press(BBImagerMessage::PushScreen(Screen::BoardSelection(
            Default::default(),
        )));

        let choose_image_btn = match selected_image {
            Some(x) => home_btn_text(x.to_string(), true, iced::Length::Fill),
            None => home_btn_text("CHOOSE IMAGE", selected_board.is_some(), iced::Length::Fill),
        }
        .width(iced::Length::Fill)
        .on_press_maybe(selected_board.map(|board| {
            BBImagerMessage::PushScreen(Screen::ImageSelection(ImageSelectionState::new(
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
            Some(BBImagerMessage::PushScreen(Screen::DestinationSelection(
                Default::default(),
            )))
        });

        let reset_btn = home_btn_text("RESET", true, iced::Length::Fill)
            .on_press(BBImagerMessage::Reset)
            .width(iced::Length::Fill);

        let config_btn = home_btn_svg(constants::SETTINGS_ICON, true).on_press(
            BBImagerMessage::PushScreen(Screen::ExtraConfiguration(Default::default())),
        );

        let next_btn_active =
            selected_board.is_none() || selected_image.is_none() || selected_dst.is_none();
        let next_btn = home_btn_text("WRITE", !next_btn_active, iced::Length::Fill)
            .width(iced::Length::Fill)
            .on_press_maybe(if next_btn_active {
                None
            } else {
                Some(BBImagerMessage::WriteBtn)
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

        // Check if BeagleV-Fire is selected
        let show_beaglev_fire_instructions = selected_board
            .map(|board| board.name.to_lowercase().contains("beaglev-fire"))
            .unwrap_or(false);

        let instructions_widget = if show_beaglev_fire_instructions {
            Some(widget::container(
                text(constants::BEAGLEV_FIRE_INSTRUCTIONS)
                    .size(16)
                    .color(iced::Color::BLACK),
            ))
        } else {
            None
        };

        let mut inner_col = widget::column![choice_btn_row.height(iced::Length::FillPortion(1))];
        if let Some(instr) = instructions_widget {
            inner_col = inner_col.push(instr);
        }
        inner_col = inner_col.push(action_btn_row.height(iced::Length::FillPortion(1)));

        widget::column![
            widget::container(
                widget::image(widget::image::Handle::from_bytes(constants::BB_BANNER))
                    .width(size.width * 0.45)
                    .height(size.height / 4.0),
            )
            .padding(iced::Padding::new(0.0).left(40))
            .width(iced::Length::Fill),
            widget::center(
                inner_col
                    .height(iced::Length::Fill)
                    .align_x(iced::Alignment::Center),
            )
            .padding([0.0, size.width * 0.05])
            .style(|_| widget::container::background(constants::BEAGLE_BRAND_COLOR)),
        ]
        .into()
    })
    .into()
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
