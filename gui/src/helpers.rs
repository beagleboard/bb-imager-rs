use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use iced::{
    widget::{self, button, text},
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

#[derive(Debug, Clone)]
pub struct Device {
    pub description: String,
    pub icon: url::Url,
    pub flasher: bb_imager::config::Flasher,
    pub documentation: url::Url,
}

impl From<bb_imager::config::Device> for Device {
    fn from(value: bb_imager::config::Device) -> Self {
        Self {
            description: value.description,
            icon: value.icon,
            flasher: value.flasher,
            documentation: value.documentation,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Image {
    pub name: String,
    pub description: String,
    pub icon: url::Url,
    pub url: url::Url,
    pub release_date: chrono::NaiveDate,
    pub image_sha256: [u8; 32],
    pub tags: HashSet<String>,
}

impl From<bb_imager::config::OsList> for Image {
    fn from(value: bb_imager::config::OsList) -> Self {
        Self {
            name: value.name,
            description: value.description,
            icon: value.icon,
            url: value.url,
            release_date: value.release_date,
            image_sha256: value.image_sha256,
            tags: value.tags,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Boards(HashMap<String, (Device, Vec<Image>)>);

impl Boards {
    pub fn merge(mut self, config: bb_imager::config::Config) -> Self {
        for dev in config.imager.devices {
            if !self.0.contains_key(&dev.name) {
                let temp = self.0.insert(dev.name.clone(), (dev.into(), Vec::new()));
                assert!(temp.is_none());
            }
        }

        for image in config.os_list {
            for board in image.devices.iter() {
                match self.0.get_mut(board) {
                    Some(val) => val.1.push(Image::from(image.clone())),
                    None => tracing::warn!("Unknown Board: {}", board),
                }
            }
        }

        self
    }

    pub fn devices(&self) -> impl Iterator<Item = (&str, &Device)> {
        self.0.iter().map(|(x, (y, _))| (x.as_str(), y))
    }

    pub fn images<'a>(&'a self, board: &'a str) -> impl Iterator<Item = &'a Image> {
        self.0.get(board).expect("Board does not exist").1.iter()
    }

    pub fn device<'a>(&'a self, board: &'a str) -> &'a Device {
        &self.0.get(board).expect("Board does not exist").0
    }
}

impl From<bb_imager::config::Config> for Boards {
    fn from(value: bb_imager::config::Config) -> Self {
        let mut ans: HashMap<String, (Device, Vec<Image>)> = value
            .imager
            .devices
            .into_iter()
            .map(|x| (x.name.clone(), (x.into(), Vec::new())))
            .collect();

        for image in value.os_list {
            for board in image.devices.iter() {
                match ans.get_mut(board) {
                    Some(val) => val.1.push(Image::from(image.clone())),
                    None => tracing::warn!("Unknown Board: {}", board),
                }
            }
        }

        Self(ans)
    }
}

impl From<&Image> for bb_imager::SelectedImage {
    fn from(value: &Image) -> Self {
        Self::remote(value.name.clone(), value.url.clone(), value.image_sha256)
    }
}

pub fn home_btn<'a>(
    txt: impl text::IntoFragment<'a>,
    active: bool,
    text_width: iced::Length,
) -> widget::Button<'a, BBImagerMessage> {
    let btn = button(
        text(txt)
            .font(constants::FONT_BOLD)
            .align_x(iced::Alignment::Center)
            .align_y(iced::Alignment::Center)
            .width(text_width),
    )
    .padding(16);

    let style = if active {
        widget::button::Style {
            background: Some(iced::Color::WHITE.into()),
            text_color: iced::Color::parse("#aa5137").unwrap(),
            ..Default::default()
        }
    } else {
        widget::button::Style {
            background: Some(iced::Color::BLACK.scale_alpha(0.5).into()),
            text_color: iced::Color::BLACK.scale_alpha(0.8),
            ..Default::default()
        }
    };

    btn.style(move |_, _| style)
}
