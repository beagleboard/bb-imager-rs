use crate::{BBImager, message::BBImagerMessage};

mod board_selection;
mod configuration;
mod destination_selection;
mod flash;
mod flash_finish;
mod helpers;
mod image_selection;

pub(crate) fn view(state: &BBImager) -> iced::Element<'_, BBImagerMessage> {
    match state {
        BBImager::ChooseBoard(inner) => board_selection::view(inner),
        BBImager::ChooseOs(inner) => image_selection::view(inner),
        BBImager::ChooseDest(inner) => destination_selection::view(inner),
        BBImager::Customize(inner) => configuration::view(inner),
        BBImager::Review(inner) => {
            bb_imager_ui::review::view(&inner.state, inner.common.scroll_id.clone())
                .map(BBImagerMessage::UiState)
        }
        BBImager::Flashing(inner) => flash::view(inner),
        BBImager::FlashingCancel(inner) => flash_finish::cancel(inner),
        BBImager::FlashingFail(inner) => flash_finish::fail(inner),
        BBImager::FlashingSuccess(inner) => flash_finish::success(inner),
        BBImager::AppInfo(inner) => {
            bb_imager_ui::app_info::view(&inner.state, inner.common().scroll_id.clone())
                .map(BBImagerMessage::UiState)
        }
        _ => panic!("Unexpected message"),
    }
}
