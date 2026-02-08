use iced::{
    Element,
    widget::{self, button},
};

use crate::{
    BBImagerMessage, FlashingState, constants,
    ui::helpers::{self, ProgressCircle, detail_entry, page_type1},
};

pub(crate) fn view(state: &FlashingState) -> Element<'_, BBImagerMessage> {
    page_type1(
        &state.common,
        info_view(state),
        progress_view(state),
        [button("Cancel")
            .style(widget::button::danger)
            .on_press(BBImagerMessage::FlashCancel)],
    )
}

pub(crate) fn progress_view(state: &FlashingState) -> Element<'_, BBImagerMessage> {
    let (prog, label) = match state.progress {
        bb_flasher::DownloadFlashingStatus::Preparing => (0.0, "Preparing ..."),
        bb_flasher::DownloadFlashingStatus::DownloadingProgress(x) => (x, "Downloading ..."),
        bb_flasher::DownloadFlashingStatus::FlashingProgress(x) => (x, "Flashing Image ..."),
        bb_flasher::DownloadFlashingStatus::Verifying => (0.99, "Verifying ..."),
        bb_flasher::DownloadFlashingStatus::Customizing => (0.99, "Customizing ..."),
    };

    let progress = ProgressCircle::new(prog, 10.0, constants::TONGUE_ORANGE);

    let col = widget::column![progress, widget::text(label)]
        .align_x(iced::Center)
        .padding(16);

    let col = match state.time_remaining() {
        Some(x) => col.push(detail_entry(
            "Time Remaining",
            crate::helpers::pretty_duration(x),
        )),
        None => col,
    };

    col.into()
}

pub(crate) fn info_view(state: &FlashingState) -> Element<'_, BBImagerMessage> {
    helpers::board_view_pane(state.selected_board(), &state.common)
}
