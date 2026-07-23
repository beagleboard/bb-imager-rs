use bb_iced_widgets::progress_circle;
use iced::{
    Element,
    widget::{self, button},
};

use crate::constants::{FONT_BOLD, TONGUE_ORANGE};
use crate::ui::helpers::{self, VIEW_COL_PADDING, detail_entry, page_type1};
use crate::{BBImagerMessage, state::FlashingState};

pub(crate) fn view(state: &FlashingState) -> Element<'_, BBImagerMessage> {
    page_type1(
        helpers::board_view_pane(&state.selected_board, &state.common),
        progress_view(state),
        [button("Cancel")
            .style(widget::button::danger)
            .on_press(BBImagerMessage::FlashCancel)],
    )
}

fn progress_view(state: &FlashingState) -> Element<'_, BBImagerMessage> {
    let (prog, label) = match state.progress {
        bb_flasher::DownloadFlashingStatus::Preparing => (0.0, "Preparing ..."),
        bb_flasher::DownloadFlashingStatus::DownloadingProgress(x) => (x, "Downloading ..."),
        bb_flasher::DownloadFlashingStatus::FlashingProgress(x) => (x, "Flashing Image ..."),
        bb_flasher::DownloadFlashingStatus::Verifying => (0.99, "Verifying ..."),
        bb_flasher::DownloadFlashingStatus::Customizing => (0.99, "Customizing ..."),
    };

    let progress = progress_circle(prog, 10.0f32, TONGUE_ORANGE, FONT_BOLD);

    let mut col = widget::column![progress, widget::text(label)];
    if let Some(x) = state.time_remaining() {
        col = col.push(detail_entry(
            "Time Remaining",
            crate::helpers::pretty_duration(x),
        ));
    }

    col.align_x(iced::Center).padding(VIEW_COL_PADDING).into()
}
