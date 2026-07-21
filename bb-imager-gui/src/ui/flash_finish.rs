use iced::Element;
use iced::widget::{self, button};

use crate::state::{FlashingFailState, FlashingFinishState};
use crate::ui::helpers::{board_view_pane, page_type1, progress_finish_view, selectable_text};
use crate::{BBImagerMessage, constants};

pub(crate) fn fail(state: &FlashingFailState) -> Element<'_, BBImagerMessage> {
    page_type1(
        info_view(state),
        progress_finish_view("Failed", constants::DANGER, &state.err),
        [
            button("Flash New")
                .style(widget::button::danger)
                .on_press(BBImagerMessage::Restart),
            button("Retry")
                .style(widget::button::primary)
                .on_press(BBImagerMessage::Retry),
        ],
    )
}

pub(crate) fn info_view(state: &FlashingFailState) -> Element<'_, BBImagerMessage> {
    widget::column![
        widget::text("Logs").size(28).font(constants::FONT_BOLD),
        widget::rule::horizontal(2),
        selectable_text(&state.logs)
    ]
    .spacing(8)
    .padding(crate::ui::helpers::VIEW_COL_PADDING)
    .into()
}

pub(crate) fn cancel(state: &FlashingFinishState) -> Element<'_, BBImagerMessage> {
    page_type1(
        board_view_pane(&state.selected_board, &state.common),
        progress_finish_view(
            "Cancelled",
            constants::DANGER,
            "Flashing Cancelled by the user",
        ),
        [button("Restart")
            .style(widget::button::danger)
            .on_press(BBImagerMessage::Restart)],
    )
}

pub(crate) fn success(state: &FlashingFinishState) -> Element<'_, BBImagerMessage> {
    let msg = if state.is_download {
        "Successfully Downloaded Image"
    } else {
        "Successfully Flashed Image"
    };

    page_type1(
        board_view_pane(&state.selected_board, &state.common),
        progress_finish_view("100%", constants::CHECK_MARK_GREEN, msg),
        [button("Flash Another")
            .style(widget::button::primary)
            .on_press(BBImagerMessage::Restart)],
    )
}
