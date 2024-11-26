use iced::{widget, Element};

use crate::{
    constants,
    helpers::{self, home_btn, ProgressBarState},
    BBImagerMessage, Screen,
};

#[derive(Debug, Clone)]
pub struct FlashingState {
    pub(crate) progress: ProgressBarState,
    documentation: String,
}

impl FlashingState {
    pub fn new(progress: ProgressBarState, documentation: String) -> Self {
        Self {
            documentation,
            progress,
        }
    }

    pub fn update(mut self, progress: ProgressBarState) -> Self {
        self.progress = progress;
        self
    }

    fn about(&self) -> widget::Container<'_, BBImagerMessage> {
        widget::container(widget::scrollable(widget::rich_text![
            widget::span(constants::BEAGLE_BOARD_ABOUT)
                .link(BBImagerMessage::OpenUrl(
                    "https://www.beagleboard.org/about".into()
                ))
                .color(iced::Color::WHITE),
            widget::span("\n\n"),
            widget::span("For more information, check out our documentation")
                .link(BBImagerMessage::OpenUrl(self.documentation.clone().into()))
                .color(iced::Color::WHITE)
        ]))
        .padding(32)
    }
}

pub fn view(state: &FlashingState, running: bool) -> Element<BBImagerMessage> {
    widget::responsive(move |size| {
        let prog_bar = state.progress.bar();

        let btn = if running {
            home_btn("CANCEL", true, iced::Length::Shrink).on_press(BBImagerMessage::CancelFlashing)
        } else {
            home_btn("HOME", true, iced::Length::Shrink)
                .on_press(BBImagerMessage::SwitchScreen(Screen::Home))
        };

        let bottom = widget::container(
            widget::column![state.about().height(size.height - 410.0), btn, prog_bar]
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .align_x(iced::Alignment::Center),
        )
        .style(|_| {
            widget::container::background(iced::Color::parse("#aa5137").expect("unexpected error"))
        });

        widget::column![helpers::logo(), bottom]
            .spacing(10)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .align_x(iced::Alignment::Center)
            .into()
    })
    .into()
}
