use crate::{BBImager, message::BBImagerMessage};

mod destination_selection;
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
        BBImager::Flashing(inner) => bb_imager_ui::flashing::view(
            &inner.common.img_handle_cache,
            &inner.state,
            inner.common.scroll_id.clone(),
        )
        .map(BBImagerMessage::UiState),
        BBImager::FlashingCancel(inner) => bb_imager_ui::flash_cancel::view(
            &inner.common.img_handle_cache,
            &inner.state,
            inner.common.scroll_id.clone(),
        )
        .map(BBImagerMessage::UiState),
        BBImager::FlashingFail(inner) => {
            bb_imager_ui::flash_fail::view(&inner.state).map(BBImagerMessage::UiState)
        }
        BBImager::FlashingSuccess(inner) => bb_imager_ui::flash_success::view(
            &inner.common.img_handle_cache,
            &inner.state,
            inner.common.scroll_id.clone(),
        )
        .map(BBImagerMessage::UiState),
        BBImager::AppInfo(inner) => {
            bb_imager_ui::app_info::view(&inner.state, inner.common().scroll_id.clone())
                .map(BBImagerMessage::UiState)
        }
        _ => panic!("Unexpected message"),
    }
}
