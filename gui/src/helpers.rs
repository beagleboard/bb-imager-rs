use std::borrow::Cow;

use iced::{
    widget::{self, text},
    Element,
};

use crate::{constants, BBImagerMessage};

pub fn input_with_label<'a, F>(
    label: &'static str,
    placeholder: &'static str,
    val: &'a str,
    func: F,
) -> widget::Row<'a, BBImagerMessage>
where
    F: 'a + Fn(String) -> bb_imager::FlashingConfig,
{
    element_with_label(
        label,
        widget::text_input(placeholder, val)
            .on_input(move |inp| BBImagerMessage::UpdateFlashConfig(func(inp)))
            .width(200)
            .into(),
    )
}

pub fn element_with_label<'a>(
    label: &'static str,
    el: Element<'a, BBImagerMessage>,
) -> widget::Row<'a, BBImagerMessage> {
    widget::row![text(label), widget::horizontal_space(), el]
        .padding(10)
        .spacing(10)
        .align_y(iced::Alignment::Center)
}

#[derive(Clone, Debug, Default)]
pub struct ProgressBarState {
    label: Cow<'static, str>,
    progress: f32,
    state: ProgressBarStatus,
}

impl ProgressBarState {
    pub const FLASHING_SUCCESS: Self =
        Self::const_new("Flashing Successful", 1.0, ProgressBarStatus::Success);
    pub const PREPARING: Self = Self::loading("Preparing...");
    pub const VERIFYING: Self = Self::loading("Verifying...");

    const fn const_new(label: &'static str, progress: f32, state: ProgressBarStatus) -> Self {
        Self {
            label: Cow::Borrowed(label),
            progress,
            state,
        }
    }

    pub fn content(&self) -> String {
        self.label.to_string()
    }

    pub fn running(&self) -> bool {
        self.state != ProgressBarStatus::Fail || self.state != ProgressBarStatus::Success
    }

    fn new(label: impl Into<Cow<'static, str>>, progress: f32, state: ProgressBarStatus) -> Self {
        Self {
            label: label.into(),
            progress,
            state,
        }
    }

    /// Progress should be between 0 to 1.0
    fn progress(prefix: &'static str, progress: f32) -> Self {
        Self::new(
            format!("{prefix}... {}%", (progress * 100.0).round() as usize),
            progress,
            ProgressBarStatus::Normal,
        )
    }

    const fn loading(label: &'static str) -> Self {
        Self::const_new(label, 0.5, ProgressBarStatus::Loading)
    }

    pub fn fail(label: impl Into<Cow<'static, str>>) -> Self {
        Self::new(label, 1.0, ProgressBarStatus::Fail)
    }

    pub fn bar(&self) -> widget::Column<'_, BBImagerMessage> {
        use std::ops::RangeInclusive;
        use widget::progress_bar;

        const RANGE: RangeInclusive<f32> = (0.0)..=1.0;

        widget::column![
            text(self.label.clone()).color(iced::Color::WHITE),
            progress_bar(RANGE, self.progress)
                .height(10)
                .style(self.state.style()),
        ]
        .align_x(iced::Alignment::Center)
        .padding(30)
        .spacing(10)
    }
}

impl From<bb_imager::DownloadFlashingStatus> for ProgressBarState {
    fn from(value: bb_imager::DownloadFlashingStatus) -> Self {
        match value {
            bb_imager::DownloadFlashingStatus::Preparing => Self::PREPARING,
            bb_imager::DownloadFlashingStatus::DownloadingProgress(p) => {
                Self::progress("Downloading Image", p)
            }
            bb_imager::DownloadFlashingStatus::FlashingProgress(p) => Self::progress("Flashing", p),
            bb_imager::DownloadFlashingStatus::Verifying => Self::VERIFYING,
            bb_imager::DownloadFlashingStatus::VerifyingProgress(p) => {
                Self::progress("Verifying", p)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProgressBarStatus {
    #[default]
    Normal,
    Success,
    Fail,
    Loading,
}

impl ProgressBarStatus {
    fn style(&self) -> impl Fn(&widget::Theme) -> widget::progress_bar::Style {
        match self {
            ProgressBarStatus::Normal => widget::progress_bar::primary,
            ProgressBarStatus::Success => widget::progress_bar::success,
            ProgressBarStatus::Fail => widget::progress_bar::danger,
            ProgressBarStatus::Loading => widget::progress_bar::primary,
        }
    }
}

pub fn logo<'a>() -> widget::Container<'a, BBImagerMessage> {
    widget::container(
        widget::image(widget::image::Handle::from_bytes(constants::BB_BANNER)).width(500),
    )
    .padding(64)
    .width(iced::Length::Fill)
}
