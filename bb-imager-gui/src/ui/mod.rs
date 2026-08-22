use crate::{BBImager, message::BBImagerMessage};

pub(crate) fn view(state: &BBImager) -> iced::Element<'_, BBImagerMessage> {
    match state {
        BBImager::ChooseBoard(inner) => bb_imager_ui::board_selection::view(
            &inner.common.img_handle_cache,
            &inner.inner,
            inner.common.scroll_id.clone(),
        ),
        BBImager::ChooseOs(inner) => bb_imager_ui::img_selection::view(
            &inner.common.img_handle_cache,
            &inner.inner,
            inner.common.scroll_id.clone(),
        ),
        BBImager::ChooseDest(inner) => {
            bb_imager_ui::dest_selection::view(&inner.inner, inner.common.scroll_id.clone())
        }
        BBImager::Customize(inner) => {
            bb_imager_ui::customization::view(&inner.inner, inner.common.scroll_id.clone())
        }
        BBImager::Review(inner) => {
            bb_imager_ui::review::view(&inner.inner, inner.common.scroll_id.clone())
        }
        BBImager::Flashing(inner) => bb_imager_ui::flashing::view(&inner.inner),
        BBImager::FlashingCancel(inner) => bb_imager_ui::flash_cancel::view(&inner.inner),
        BBImager::FlashingFail(inner) => bb_imager_ui::flash_fail::view(&inner.inner),
        BBImager::FlashingSuccess(inner) => {
            bb_imager_ui::flash_success::view(&inner.inner, inner.common.scroll_id.clone())
        }
        BBImager::AppInfo(inner) => {
            bb_imager_ui::app_options::view(&inner.inner, inner.common().scroll_id.clone())
        }
        _ => panic!("Unexpected message"),
    }
    .map(Into::into)
}
