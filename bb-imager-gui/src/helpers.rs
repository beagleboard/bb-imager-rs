use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use bb_imager::DownloadFlashingStatus;
use iced::{
    widget::{self, button, text},
    Element,
};

use crate::{constants, BBImagerMessage};

const ICON_SIZE: u16 = 32;
const PADDING: u16 = 4;
const RADIUS: u16 = (ICON_SIZE + PADDING * 2) / 2;

pub fn input_with_label<'a, F>(
    label: &'static str,
    placeholder: &'static str,
    val: &'a str,
    func: F,
) -> widget::Row<'a, BBImagerMessage>
where
    F: 'a + Fn(String) -> crate::pages::configuration::FlashingCustomization,
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
    inner_state: Option<DownloadFlashingStatus>,
}

impl ProgressBarState {
    pub const FLASHING_SUCCESS: Self =
        Self::const_new("Flashing Successful", 1.0, ProgressBarStatus::Success, None);
    pub const PREPARING: Self = Self::loading("Preparing...", DownloadFlashingStatus::Preparing);
    pub const VERIFYING: Self = Self::loading("Verifying...", DownloadFlashingStatus::Verifying);
    pub const CUSTOMIZING: Self =
        Self::loading("Customizing...", DownloadFlashingStatus::Customizing);

    const fn const_new(
        label: &'static str,
        progress: f32,
        state: ProgressBarStatus,
        inner_state: Option<DownloadFlashingStatus>,
    ) -> Self {
        Self {
            label: Cow::Borrowed(label),
            progress,
            state,
            inner_state,
        }
    }

    pub fn content(&self) -> String {
        self.label.to_string()
    }

    fn new(
        label: impl Into<Cow<'static, str>>,
        progress: f32,
        state: ProgressBarStatus,
        inner_state: Option<DownloadFlashingStatus>,
    ) -> Self {
        Self {
            label: label.into(),
            progress,
            state,
            inner_state,
        }
    }

    /// Progress should be between 0 to 1.0
    fn progress(prefix: &'static str, progress: f32, inner_state: DownloadFlashingStatus) -> Self {
        Self::new(
            format!("{prefix}... {}%", (progress * 100.0).round() as usize),
            progress,
            ProgressBarStatus::Normal,
            Some(inner_state),
        )
    }

    const fn loading(label: &'static str, inner_state: DownloadFlashingStatus) -> Self {
        Self::const_new(label, 0.5, ProgressBarStatus::Loading, Some(inner_state))
    }

    pub fn fail(label: impl Into<Cow<'static, str>>) -> Self {
        Self::new(label, 1.0, ProgressBarStatus::Fail, None)
    }

    pub fn bar(&self) -> widget::Column<'_, BBImagerMessage> {
        use std::ops::RangeInclusive;
        use widget::progress_bar;

        const RANGE: RangeInclusive<f32> = (0.0)..=1.0;

        widget::column![
            text(self.label.clone()).color(iced::Color::WHITE),
            progress_bar(RANGE, self.progress)
                .height(8)
                .style(self.state.style()),
        ]
        .align_x(iced::Alignment::Center)
    }

    pub fn cancel(&self) -> Option<Self> {
        let x = match self.inner_state? {
            DownloadFlashingStatus::Preparing => Self::fail("Preparation cancelled by user"),
            DownloadFlashingStatus::DownloadingProgress(_) => {
                Self::fail("Downloading cancelled by user")
            }
            DownloadFlashingStatus::FlashingProgress(_) => Self::fail("Flashing cancelled by user"),
            DownloadFlashingStatus::Verifying | DownloadFlashingStatus::VerifyingProgress(_) => {
                Self::fail("Verification cancelled by user")
            }
            DownloadFlashingStatus::Customizing => Self::fail("Customization cancelled by user"),
        };
        Some(x)
    }
}

impl From<DownloadFlashingStatus> for ProgressBarState {
    fn from(value: DownloadFlashingStatus) -> Self {
        match value {
            DownloadFlashingStatus::Preparing => Self::PREPARING,
            DownloadFlashingStatus::DownloadingProgress(p) => Self::progress(
                "Downloading Image",
                p,
                DownloadFlashingStatus::DownloadingProgress(0.0),
            ),
            DownloadFlashingStatus::FlashingProgress(p) => {
                Self::progress("Flashing", p, DownloadFlashingStatus::FlashingProgress(0.0))
            }
            DownloadFlashingStatus::Verifying => Self::VERIFYING,
            DownloadFlashingStatus::VerifyingProgress(p) => {
                Self::progress("Verifying", p, DownloadFlashingStatus::Verifying)
            }
            DownloadFlashingStatus::Customizing => Self::CUSTOMIZING,
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

#[derive(Debug, Clone)]
pub struct Device {
    pub description: String,
    pub icon: url::Url,
    pub flasher: bb_imager::Flasher,
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

pub fn home_btn_text<'a>(
    txt: impl text::IntoFragment<'a>,
    active: bool,
    text_width: iced::Length,
) -> widget::Button<'a, BBImagerMessage> {
    fn style(active: bool) -> widget::button::Style {
        if active {
            widget::button::Style {
                background: Some(iced::Color::WHITE.into()),
                text_color: constants::BEAGLE_BRAND_COLOR,
                border: iced::border::rounded(4),
                ..Default::default()
            }
        } else {
            widget::button::Style {
                background: Some(iced::Color::BLACK.scale_alpha(0.5).into()),
                text_color: iced::Color::BLACK.scale_alpha(0.8),
                border: iced::border::rounded(4),
                ..Default::default()
            }
        }
    }

    button(
        text(txt)
            .font(constants::FONT_BOLD)
            .align_x(iced::Alignment::Center)
            .align_y(iced::Alignment::Center)
            .width(text_width),
    )
    .padding(8)
    .style(move |_, _| style(active))
}

pub fn home_btn_svg<'a>(icon: &'static [u8], active: bool) -> widget::Button<'a, BBImagerMessage> {
    fn svg_style(active: bool) -> widget::svg::Style {
        if active {
            Default::default()
        } else {
            widget::svg::Style {
                color: Some(iced::Color::BLACK.scale_alpha(0.5)),
            }
        }
    }

    fn btn_style(active: bool) -> widget::button::Style {
        if active {
            widget::button::Style {
                background: Some(iced::Color::WHITE.into()),
                border: iced::border::rounded(RADIUS),
                ..Default::default()
            }
        } else {
            widget::button::Style {
                background: Some(iced::Color::BLACK.scale_alpha(0.5).into()),
                border: iced::border::rounded(RADIUS),
                ..Default::default()
            }
        }
    }

    button(
        widget::svg(widget::svg::Handle::from_memory(icon))
            .style(move |_, _| svg_style(active))
            .width(ICON_SIZE)
            .height(ICON_SIZE),
    )
    .style(move |_, _| btn_style(active))
    .padding(PADDING)
}

pub fn img_or_svg<'a>(path: std::path::PathBuf, width: u16) -> Element<'a, BBImagerMessage> {
    let img = std::fs::read(path).expect("Failed to open image");

    match image::guess_format(&img) {
        Ok(_) => widget::image(widget::image::Handle::from_bytes(img))
            .width(width)
            .height(width)
            .into(),

        Err(_) => widget::svg(widget::svg::Handle::from_memory(img))
            .width(width)
            .height(width)
            .into(),
    }
}

pub fn search_bar(refresh: Option<BBImagerMessage>, cur_search: &str) -> Element<BBImagerMessage> {
    let mut row = widget::row![button(
        widget::svg(widget::svg::Handle::from_memory(constants::ARROW_BACK_ICON)).width(22)
    )
    .on_press(BBImagerMessage::SwitchScreen(crate::pages::Screen::Home))
    .style(widget::button::secondary)]
    .spacing(10);

    if let Some(r) = refresh {
        row = row.push(
            button(
                widget::svg(widget::svg::Handle::from_memory(constants::REFRESH_ICON)).width(22),
            )
            .on_press(r)
            .style(widget::button::secondary),
        );
    }

    row.push(widget::text_input("Search", cur_search).on_input(BBImagerMessage::Search))
        .into()
}

#[derive(Debug, Clone, PartialEq)]
pub enum BoardImage {
    SdFormat,
    Image(bb_imager::common::SelectedImage),
}

impl std::fmt::Display for BoardImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoardImage::SdFormat => write!(f, "Format SD Card"),
            BoardImage::Image(selected_image) => selected_image.fmt(f),
        }
    }
}

#[derive(Debug)]
pub struct Tainted<T> {
    inner: T,
    flag: bool,
}

impl<T> Tainted<T> {
    pub const fn new(inner: T) -> Self {
        Self { inner, flag: false }
    }

    pub const fn new_tainted(inner: T) -> Self {
        Self { inner, flag: true }
    }

    pub const fn is_tainted(&self) -> bool {
        self.flag
    }
}

impl<T> AsRef<T> for Tainted<T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}
