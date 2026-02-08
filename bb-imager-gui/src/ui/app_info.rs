use iced::{Element, widget};

use crate::{
    OverlayState,
    message::BBImagerMessage,
    ui::helpers::{element_with_label, page_type3, selectable_text},
};

const INP_BOX_WIDTH: u32 = 420;

pub(crate) fn view<'a>(state: &'a OverlayState) -> Element<'a, BBImagerMessage> {
    page_type3(
        review_view(state),
        [widget::button("BACK")
            .on_press(BBImagerMessage::Back)
            .style(widget::button::secondary)],
    )
}

fn review_view<'a>(state: &'a OverlayState) -> Element<'a, BBImagerMessage> {
    let col = widget::column![
        widget::image(state.common().window_icon().clone()),
        crate::constants::APP_NAME,
        crate::constants::APP_RELEASE,
        crate::constants::APP_DESC,
        widget::rule::horizontal(2),
        element_with_label(
            "Cache Directory",
            widget::text_input(state.cache_dir_path(), state.cache_dir_path())
                .width(INP_BOX_WIDTH)
                .on_input(|_| BBImagerMessage::Null)
                .into()
        ),
        widget::rule::horizontal(2),
        element_with_label(
            "Log File",
            widget::text_input(state.log_path(), state.log_path())
                .width(INP_BOX_WIDTH)
                .on_input(|_| BBImagerMessage::Null)
                .into()
        ),
        widget::rule::horizontal(2),
        widget::container(selectable_text(&state.license)).padding(iced::Padding::ZERO.right(16))
    ]
    .spacing(8)
    .width(iced::Fill)
    .align_x(iced::Center);

    widget::scrollable(col).into()
}
