use crate::{BBImager, message::BBImagerMessage};

pub(crate) fn view(state: &BBImager) -> iced::Element<'_, BBImagerMessage> {
    match state {
        BBImager::ChooseBoard(inner) => bb_imager_ui::board_selection::view(
            &inner.common.img_handle_cache,
            &inner.state,
            inner.common.scroll_id.clone(),
        ),
        BBImager::ChooseOs(inner) => bb_imager_ui::image_selection::view(
            &inner.common.img_handle_cache,
            &inner.state,
            inner.common.scroll_id.clone(),
        ),
        BBImager::ChooseDest(inner) => {
            bb_imager_ui::destination_selection::view(&inner.state, inner.common.scroll_id.clone())
        }
        BBImager::Customize(inner) => {
            bb_imager_ui::configuration::view(&inner.state, inner.common.scroll_id.clone())
        }
        BBImager::Review(inner) => {
            bb_imager_ui::review::view(&inner.state, inner.common.scroll_id.clone())
        }
        BBImager::Flashing(inner) => bb_imager_ui::flashing::view(
            &inner.common.img_handle_cache,
            &inner.state,
            inner.common.scroll_id.clone(),
        ),
        BBImager::FlashingCancel(inner) => bb_imager_ui::flash_cancel::view(
            &inner.common.img_handle_cache,
            &inner.state,
            inner.common.scroll_id.clone(),
        ),
        BBImager::FlashingFail(inner) => bb_imager_ui::flash_fail::view(&inner.state),
        BBImager::FlashingSuccess(inner) => bb_imager_ui::flash_success::view(
            &inner.common.img_handle_cache,
            &inner.state,
            inner.common.scroll_id.clone(),
        ),
        BBImager::AppInfo(inner) => {
            bb_imager_ui::app_info::view(&inner.state, inner.common().scroll_id.clone())
        }
        BBImager::SandboxNotice(inner) => {
            bb_imager_ui::sandbox_notice::view(&inner.state, inner.common.scroll_id.clone())
        }
        _ => panic!("Unexpected message"),
    }
    .map(BBImagerMessage::UiState)
}
