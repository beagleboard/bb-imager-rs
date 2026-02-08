use crate::{BBImager, message::BBImagerMessage};

mod app_info;
mod board_selection;
mod configuration;
mod destination_selection;
mod flash;
mod flash_cancel;
mod flash_fail;
mod flash_success;
mod helpers;
mod image_selection;
mod review;

pub(crate) fn view(state: &BBImager) -> iced::Element<'_, BBImagerMessage> {
    match state {
        BBImager::ChooseBoard(inner) => board_selection::view(inner),
        BBImager::ChooseOs(inner) => image_selection::view(inner),
        BBImager::ChooseDest(inner) => destination_selection::view(inner),
        BBImager::Customize(inner) => configuration::view(inner),
        BBImager::Review(inner) => review::view(inner),
        BBImager::Flashing(inner) => flash::view(inner),
        BBImager::FlashingCancel(inner) => flash_cancel::view(inner),
        BBImager::FlashingFail(inner) => flash_fail::view(inner),
        BBImager::FlashingSuccess(inner) => flash_success::view(inner),
        BBImager::AppInfo(inner) => app_info::view(inner),
        _ => panic!("Unexpected message"),
    }
}
