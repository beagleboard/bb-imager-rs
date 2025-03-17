use iced::{Element, widget};

use crate::{
    BBImagerMessage, Screen, constants,
    helpers::{ProgressBarState, home_btn_text},
};

#[derive(Debug, Clone)]
pub(crate) struct FlashingState {
    pub(crate) progress: ProgressBarState,
    documentation: String,
}

impl FlashingState {
    pub(crate) fn new(progress: ProgressBarState, documentation: String) -> Self {
        Self {
            documentation,
            progress,
        }
    }

    pub(crate) fn update(mut self, progress: ProgressBarState) -> Self {
        self.progress = progress;
        self
    }

    fn about(&self) -> widget::Scrollable<'_, BBImagerMessage> {
        widget::scrollable(widget::rich_text![
            widget::span(constants::BEAGLE_BOARD_ABOUT)
                .link(BBImagerMessage::OpenUrl(
                    "https://www.beagleboard.org/about".into()
                ))
                .color(iced::Color::WHITE),
            widget::span("\n\n"),
            widget::span("For more information, check out our documentation")
                .link(BBImagerMessage::OpenUrl(self.documentation.clone().into()))
                .color(iced::Color::WHITE)
        ])
    }
}

pub(crate) fn view(state: &FlashingState, running: bool) -> Element<BBImagerMessage> {
    widget::responsive(move |size| {
        const FOOTER_HEIGHT: f32 = 150.0;
        let banner_height = size.height / 4.0;

        let prog_bar = state.progress.bar().spacing(12);

        let btn = if running {
            home_btn_text("CANCEL", true, iced::Length::Shrink)
                .on_press(BBImagerMessage::CancelFlashing)
        } else {
            home_btn_text("HOME", true, iced::Length::Shrink)
                .on_press(BBImagerMessage::SwitchScreen(Screen::Home))
        };

        let bottom = widget::container(
            widget::column![
                state
                    .about()
                    .height(size.height - FOOTER_HEIGHT - banner_height),
                btn,
                prog_bar
            ]
            .padding(16)
            .spacing(12)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .align_x(iced::Alignment::Center),
        )
        .style(|_| widget::container::background(constants::BEAGLE_BRAND_COLOR));

        widget::column![
            widget::container(
                widget::image(widget::image::Handle::from_bytes(constants::BB_BANNER))
                    .width(size.width * 0.45)
                    .height(banner_height),
            )
            .padding(iced::Padding::new(0.0).left(40))
            .width(iced::Length::Fill),
            bottom
        ]
        .spacing(10)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .align_x(iced::Alignment::Center)
        .into()
    })
    .into()
}
