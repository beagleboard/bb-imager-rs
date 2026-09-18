use crate::{BBImager, message::BBImagerMessage};

mod destination_selection;
mod flash;
mod flash_finish;
mod helpers;
mod image_selection;

pub(crate) fn view(state: &BBImager) -> iced::Element<'_, BBImagerMessage> {
    match state {
        BBImager::ChooseBoard(inner) => bb_imager_ui::board_selection::view(
            &inner.common.img_handle_cache,
            &inner.state,
            inner.common.scroll_id.clone(),
        )
        .map(BBImagerMessage::UiState),
        BBImager::ChooseOs(inner) => image_selection::view(inner),
        BBImager::ChooseDest(inner) => destination_selection::view(inner),
        BBImager::Customize(inner) => {
            bb_imager_ui::configuration::view(&inner.state, inner.common.scroll_id.clone())
                .map(BBImagerMessage::UiState)
        }
        BBImager::Review(inner) => {
            bb_imager_ui::review::view(&inner.state, inner.common.scroll_id.clone())
                .map(BBImagerMessage::UiState)
        }
        BBImager::Flashing(inner) => flash::view(inner),
        BBImager::FlashingCancel(inner) => flash_finish::cancel(inner),
        BBImager::FlashingFail(inner) => {
            bb_imager_ui::flash_fail::view(&inner.state).map(BBImagerMessage::UiState)
        }
        BBImager::FlashingSuccess(inner) => flash_finish::success(inner),
        BBImager::AppInfo(inner) => {
            bb_imager_ui::app_info::view(&inner.state, inner.common().scroll_id.clone())
                .map(BBImagerMessage::UiState)
        }
        _ => panic!("Unexpected message"),
    }
}
