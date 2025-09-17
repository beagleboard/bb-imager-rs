use iced::{Element, widget};

use crate::{BBImager, constants, message::BBImagerMessage, pages::Screen};

mod board_selection;
mod configuration;
mod destination_selection;
mod flash;
mod helpers;
mod home;
mod image_selection;

pub(crate) fn view(state: &BBImager) -> Element<BBImagerMessage> {
    tracing::debug!("Page Stack: {:#?}", state.screen);

    match state.screen.last().expect("No Screen") {
        Screen::Home => home::view(
            state.selected_device(),
            state.selected_image(),
            state.selected_destination(),
            state.is_destionation_selectable(),
            state.is_download_action(),
        ),
        Screen::BoardSelection(p) => {
            board_selection::view(state.devices(), p.search_str(), state.downloader())
        }
        Screen::ImageSelection(page) => {
            let mut extra_entries = Vec::from([image_selection::ExtraImageEntry::new(
                "Custom Image",
                constants::FILE_ADD_ICON,
                BBImagerMessage::SelectLocalImage(page.flasher()),
            )]);
            if page.flasher() == bb_config::config::Flasher::SdCard {
                extra_entries.push(image_selection::ExtraImageEntry::new(
                    "Format Sd Card",
                    constants::FORMAT_ICON,
                    BBImagerMessage::SelectImage(crate::helpers::BoardImage::SdFormat),
                ));
            }

            image_selection::view(
                page,
                state.images(page.idx()),
                state.downloader(),
                extra_entries,
            )
        }
        Screen::DestinationSelection(s) => destination_selection::view(
            state.destinations(),
            s.search_str(),
            state.selected_image().unwrap().file_name(),
        ),
        Screen::ExtraConfiguration(id) => configuration::view(
            state.app_settings(),
            state.customization(),
            state.timezones(),
            state.keymaps(),
            *id,
        ),
        Screen::Flashing(s) => flash::view(s, state.is_flashing()),
        Screen::FlashingConfirmation => {
            let base = home::view(
                state.selected_device(),
                state.selected_image(),
                state.selected_destination(),
                state.is_destionation_selectable(),
                state.is_download_action(),
            );
            dialog(base, flashing_confirmation_menu())
        }
    }
}

fn flashing_confirmation_menu<'a>() -> Element<'a, BBImagerMessage> {
    let menu = widget::column![
        widget::text("Would you like to apply customization settings?"),
        widget::row![
            widget::button("Edit Settings").on_press(BBImagerMessage::ReplaceScreen(
                Screen::ExtraConfiguration(Default::default())
            )),
            widget::button("Yes").on_press(BBImagerMessage::StartFlashing),
            widget::button("No").on_press(BBImagerMessage::StartFlashingWithoutConfiguraton),
            widget::button("Abort").on_press(BBImagerMessage::SwitchScreen(Screen::Home))
        ]
        .spacing(8)
    ]
    .align_x(iced::Alignment::Center)
    .padding(16)
    .spacing(16);

    widget::container(menu)
        .style(|_| widget::container::background(iced::Color::WHITE))
        .into()
}

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
