use iced::{widget, Element};

use crate::{
    constants,
    helpers::{self, home_btn, ProgressBarState},
    BBImagerMessage, Screen,
};

#[derive(Debug, Clone)]
pub struct FlashingScreen {
    progress: ProgressBarState,
    documentation: String,
    running: bool,
}

impl Default for FlashingScreen {
    fn default() -> Self {
        FlashingScreen {
            progress: ProgressBarState::PREPARING,
            documentation: String::new(),
            running: true,
        }
    }
}

impl FlashingScreen {
    pub fn new(documentation: String) -> Self {
        Self {
            documentation,
            ..Default::default()
        }
    }

    pub fn update_progress(mut self, progress: ProgressBarState, running: bool) -> Self {
        self.progress = progress;
        self.running = running;
        self
    }

    pub fn view(&self) -> Element<BBImagerMessage> {
        widget::responsive(|size| {
            let prog_bar = self.progress.bar();

            let btn = if self.running {
                home_btn("CANCEL", true, iced::Length::Shrink)
                    .on_press(BBImagerMessage::CancelFlashing)
            } else {
                home_btn("HOME", true, iced::Length::Shrink)
                    .on_press(BBImagerMessage::SwitchScreen(Screen::Home))
            };

            let bottom = widget::container(
                widget::column![self.about().height(size.height - 410.0), btn, prog_bar]
                    .width(iced::Length::Fill)
                    .height(iced::Length::Fill)
                    .align_x(iced::Alignment::Center),
            )
            .style(|_| {
                widget::container::background(
                    iced::Color::parse("#aa5137").expect("unexpected error"),
                )
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
