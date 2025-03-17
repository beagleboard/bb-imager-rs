use std::{borrow::Cow, collections::HashSet, io::Read, path::PathBuf, sync::LazyLock};

use bb_imager::{BBFlasher, DownloadFlashingStatus, config::OsListItem};
use futures::StreamExt;
use iced::{
    Element, futures,
    widget::{self, button, text},
};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::{BBImagerMessage, constants, pages::configuration::FlashingCustomization};

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
    F: 'a + Fn(String) -> FlashingCustomization,
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

#[derive(Debug, Clone, Default)]
pub struct Boards(bb_imager::config::Config);

impl Boards {
    pub fn merge(mut self, config: bb_imager::config::Config) -> Self {
        for dev in config.imager.devices {
            if !self.0.imager.devices.iter().any(|x| x.name == dev.name) {
                self.0.imager.devices.push(dev);
            }
        }

        self.0.os_list.extend(config.os_list);

        self
    }

    pub fn devices(&self) -> impl Iterator<Item = (usize, &bb_imager::config::Device)> {
        self.0.imager.devices.iter().enumerate()
    }

    pub fn image(&self, target: &[usize]) -> &OsListItem {
        let mut res = &self.0.os_list;
        let (last, rest) = target.split_last().unwrap();

        for i in rest {
            let item = res.get(*i).expect("No Subitem");
            res = match item {
                OsListItem::Image(_) => panic!("No subitem"),
                OsListItem::SubList { subitems, .. } => subitems,
                OsListItem::RemoteSubList { .. } => panic!("No subitem"),
            }
        }

        res.get(*last).unwrap()
    }

    pub fn images(
        &self,
        board_idx: usize,
        subitems: &[usize],
    ) -> Option<Vec<(usize, &OsListItem)>> {
        let mut res = &self.0.os_list;

        for i in subitems {
            let item = res.get(*i).expect("No Subitem");
            res = match item {
                OsListItem::Image(_) => panic!("No subitem"),
                OsListItem::SubList { subitems, .. } => subitems,
                OsListItem::RemoteSubList { .. } => return None,
            }
        }

        let dev = self.device(board_idx);
        let tags = &dev.tags;

        Some(
            res.iter()
                .enumerate()
                .filter(move |(_, x)| check_board(x, tags))
                .collect(),
        )
    }

    pub fn device(&self, board_idx: usize) -> &bb_imager::config::Device {
        self.0
            .imager
            .devices
            .get(board_idx)
            .expect("Board does not exist")
    }

    pub fn resolve_remote_subitem(&mut self, subitems: Vec<OsListItem>, target: &[usize]) {
        assert!(!target.is_empty());

        let mut res = &mut self.0.os_list;

        let (last, rest) = target.split_last().unwrap();

        for i in rest {
            let item = res.get_mut(*i).expect("No Subitem");
            res = match item {
                OsListItem::Image(_) => panic!("No subitem"),
                OsListItem::SubList { subitems, .. } => subitems,
                OsListItem::RemoteSubList { .. } => panic!("No subitem"),
            }
        }

        if let OsListItem::RemoteSubList {
            name,
            description,
            icon,
            flasher,
            ..
        } = res.get(*last).unwrap().clone()
        {
            res[*last] = OsListItem::SubList {
                name,
                description,
                icon,
                flasher,
                subitems,
            }
        } else {
            tracing::warn!("Unexpected item")
        }
    }
}

fn check_board(item: &OsListItem, tags: &HashSet<String>) -> bool {
    match item {
        OsListItem::Image(os_image) => !tags.is_disjoint(&os_image.devices),
        OsListItem::SubList { subitems, .. } => subitems.iter().any(|x| check_board(x, tags)),
        OsListItem::RemoteSubList { devices, .. } => !tags.is_disjoint(devices),
    }
}

impl From<bb_imager::config::Config> for Boards {
    fn from(value: bb_imager::config::Config) -> Self {
        Self(value)
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

pub fn search_bar(cur_search: &str) -> Element<BBImagerMessage> {
    widget::row![
        button(widget::svg(widget::svg::Handle::from_memory(constants::ARROW_BACK_ICON)).width(22))
            .on_press(BBImagerMessage::PopScreen)
            .style(widget::button::secondary),
        widget::text_input("Search", cur_search).on_input(BBImagerMessage::Search)
    ]
    .spacing(10)
    .into()
}

#[derive(Debug, Clone)]
pub enum BoardImage {
    SdFormat,
    Image {
        flasher: bb_imager::Flasher,
        img: SelectedImage,
    },
}

impl BoardImage {
    pub fn local(path: PathBuf, flasher: bb_imager::Flasher) -> Self {
        Self::Image {
            img: bb_imager::img::LocalImage::new(path).into(),
            flasher,
        }
    }

    pub fn remote(
        image: bb_imager::config::OsImage,
        flasher: bb_imager::Flasher,
        downloader: bb_downloader::Downloader,
    ) -> Self {
        Self::Image {
            img: RemoteImage::new(
                image.name,
                image.url,
                image.image_download_sha256,
                downloader,
            )
            .into(),
            flasher,
        }
    }

    pub const fn flasher(&self) -> bb_imager::Flasher {
        match self {
            BoardImage::SdFormat => bb_imager::Flasher::SdCard,
            BoardImage::Image { flasher, .. } => *flasher,
        }
    }

    pub(crate) fn is_sd_format(&self) -> bool {
        matches!(self, BoardImage::SdFormat)
    }
}

impl std::fmt::Display for BoardImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoardImage::SdFormat => write!(f, "Format SD Card"),
            BoardImage::Image { img: image, .. } => image.fmt(f),
        }
    }
}

pub fn system_timezone() -> Option<&'static String> {
    static SYSTEM_TIMEZONE: LazyLock<Option<String>> = LazyLock::new(localzone::get_local_zone);

    (*SYSTEM_TIMEZONE).as_ref()
}

/// Configuration for GUI that should be presisted
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GuiConfiguration {
    #[serde(skip_serializing_if = "Option::is_none")]
    sd_customization: Option<SdCustomization>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bcf_customization: Option<BcfCustomization>,
    #[cfg(feature = "pb2_mspm0")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pb2_mspm0_customization: Option<Pb2Mspm0Customization>,
}

impl GuiConfiguration {
    pub(crate) fn load() -> std::io::Result<Self> {
        let mut data = Vec::with_capacity(512);
        let config_p = Self::config_path().unwrap();

        let mut config = std::fs::File::open(config_p)?;
        config.read_to_end(&mut data)?;

        Ok(serde_json::from_slice(&data).unwrap())
    }

    pub(crate) async fn save(&self) -> std::io::Result<()> {
        let data = serde_json::to_string_pretty(self).unwrap();
        let config_p = Self::config_path().unwrap();

        tracing::info!("Configuration Path: {:?}", config_p);
        tokio::fs::create_dir_all(config_p.parent().unwrap()).await?;

        let mut config = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(config_p)
            .await?;

        config.write_all(data.as_bytes()).await?;

        Ok(())
    }

    fn config_path() -> Option<PathBuf> {
        let dirs = directories::ProjectDirs::from(
            constants::PACKAGE_QUALIFIER.0,
            constants::PACKAGE_QUALIFIER.1,
            constants::PACKAGE_QUALIFIER.2,
        )?;

        Some(dirs.config_local_dir().join("config.json").to_owned())
    }

    pub const fn sd_customization(&self) -> Option<&SdCustomization> {
        self.sd_customization.as_ref()
    }

    pub const fn bcf_customization(&self) -> Option<&BcfCustomization> {
        self.bcf_customization.as_ref()
    }

    #[cfg(feature = "pb2_mspm0")]
    pub const fn pb2_mspm0_customization(&self) -> Option<&Pb2Mspm0Customization> {
        self.pb2_mspm0_customization.as_ref()
    }

    pub fn update_sd_customization(&mut self, t: SdCustomization) {
        self.sd_customization = Some(t);
    }

    pub fn update_bcf_customization(&mut self, t: BcfCustomization) {
        self.bcf_customization = Some(t)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SdCustomization {
    pub(crate) verify: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) keymap: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) user: Option<SdCustomizationUser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) wifi: Option<SdCustomizationWifi>,
}

impl Default for SdCustomization {
    fn default() -> Self {
        Self {
            verify: true,
            hostname: None,
            timezone: None,
            keymap: None,
            user: None,
            wifi: None,
        }
    }
}

impl SdCustomization {
    pub(crate) fn update_verify(mut self, t: bool) -> Self {
        self.verify = t;
        self
    }

    pub(crate) fn update_hostname(mut self, t: Option<String>) -> Self {
        self.hostname = t;
        self
    }

    pub(crate) fn update_timezone(mut self, t: Option<String>) -> Self {
        self.timezone = t;
        self
    }

    pub(crate) fn update_keymap(mut self, t: Option<String>) -> Self {
        self.keymap = t;
        self
    }

    pub(crate) fn update_user(mut self, t: Option<SdCustomizationUser>) -> Self {
        self.user = t;
        self
    }

    pub(crate) fn update_wifi(mut self, t: Option<SdCustomizationWifi>) -> Self {
        self.wifi = t;
        self
    }
}

impl From<SdCustomization> for bb_imager::flasher::sd::FlashingSdLinuxConfig {
    fn from(value: SdCustomization) -> Self {
        Self::new(
            value.verify,
            value.hostname,
            value.timezone,
            value.keymap,
            value.user.map(|x| (x.username, x.password)),
            value.wifi.map(|x| (x.ssid, x.password)),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SdCustomizationUser {
    pub(crate) username: String,
    pub(crate) password: String,
}

impl SdCustomizationUser {
    pub(crate) const fn new(username: String, password: String) -> Self {
        Self { username, password }
    }

    pub(crate) fn update_username(mut self, t: String) -> Self {
        self.username = t;
        self
    }

    pub(crate) fn update_password(mut self, t: String) -> Self {
        self.password = t;
        self
    }
}

impl Default for SdCustomizationUser {
    fn default() -> Self {
        Self::new(whoami::username(), String::new())
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SdCustomizationWifi {
    pub(crate) ssid: String,
    pub(crate) password: String,
}

impl SdCustomizationWifi {
    pub(crate) fn update_ssid(mut self, t: String) -> Self {
        self.ssid = t;
        self
    }

    pub(crate) fn update_password(mut self, t: String) -> Self {
        self.password = t;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BcfCustomization {
    pub(crate) verify: bool,
}

impl BcfCustomization {
    pub(crate) fn update_verify(mut self, t: bool) -> Self {
        self.verify = t;
        self
    }
}

impl From<BcfCustomization> for bb_imager::flasher::bcf::FlashingBcfConfig {
    fn from(value: BcfCustomization) -> Self {
        Self {
            verify: value.verify,
        }
    }
}

impl Default for BcfCustomization {
    fn default() -> Self {
        Self { verify: true }
    }
}

#[cfg(feature = "pb2_mspm0")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Pb2Mspm0Customization {
    pub(crate) persist_eeprom: bool,
}

#[cfg(feature = "pb2_mspm0")]
impl Pb2Mspm0Customization {
    pub(crate) fn update_persist_eeprom(mut self, t: bool) -> Self {
        self.persist_eeprom = t;
        self
    }
}

#[cfg(feature = "pb2_mspm0")]
impl Default for Pb2Mspm0Customization {
    fn default() -> Self {
        Self {
            persist_eeprom: true,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteImage {
    name: String,
    url: url::Url,
    extract_sha256: [u8; 32],
    downloader: bb_downloader::Downloader,
}

impl RemoteImage {
    pub(crate) const fn new(
        name: String,
        url: url::Url,
        extract_sha256: [u8; 32],
        downloader: bb_downloader::Downloader,
    ) -> Self {
        Self {
            name,
            url,
            extract_sha256,
            downloader,
        }
    }
}

impl bb_imager::img::ImageFile for RemoteImage {
    async fn resolve(
        &self,
        chan: Option<futures::channel::mpsc::Sender<DownloadFlashingStatus>>,
    ) -> std::io::Result<PathBuf> {
        let (tx, rx) = futures::channel::mpsc::channel(20);

        if let Some(chan) = chan {
            tokio::spawn(async move {
                rx.map(DownloadFlashingStatus::DownloadingProgress)
                    .map(Ok)
                    .forward(chan)
                    .await
            });
        }

        self.downloader
            .download_with_sha(self.url.clone(), self.extract_sha256, Some(tx))
            .await
            .map_err(|e| {
                if let bb_downloader::Error::IoError(x) = e {
                    x
                } else {
                    std::io::Error::other(format!("Failed to open image: {e}"))
                }
            })
    }
}

impl std::fmt::Display for RemoteImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone)]
pub(crate) enum SelectedImage {
    LocalImage(bb_imager::img::LocalImage),
    RemoteImage(RemoteImage),
}

impl bb_imager::img::ImageFile for SelectedImage {
    async fn resolve(
        &self,
        chan: Option<futures::channel::mpsc::Sender<DownloadFlashingStatus>>,
    ) -> std::io::Result<PathBuf> {
        match self {
            SelectedImage::LocalImage(x) => x.resolve(chan).await,
            SelectedImage::RemoteImage(x) => x.resolve(chan).await,
        }
    }
}

impl std::fmt::Display for SelectedImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectedImage::LocalImage(x) => x.fmt(f),
            SelectedImage::RemoteImage(x) => x.fmt(f),
        }
    }
}

impl From<RemoteImage> for SelectedImage {
    fn from(value: RemoteImage) -> Self {
        Self::RemoteImage(value)
    }
}

impl From<bb_imager::img::LocalImage> for SelectedImage {
    fn from(value: bb_imager::img::LocalImage) -> Self {
        Self::LocalImage(value)
    }
}

pub(crate) async fn flash(
    img: Option<BoardImage>,
    customization: FlashingCustomization,
    dst: Option<bb_imager::Destination>,
    chan: futures::channel::mpsc::Sender<DownloadFlashingStatus>,
) -> std::io::Result<()> {
    match (img, customization, dst) {
        (
            Some(BoardImage::SdFormat),
            FlashingCustomization::LinuxSdFormat,
            Some(bb_imager::Destination::SdCard { path, .. }),
        ) => {
            bb_imager::flasher::sd::LinuxSdFormat::new(path)
                .flash(Some(chan))
                .await
        }
        (
            Some(BoardImage::Image { img, .. }),
            FlashingCustomization::LinuxSd(customization),
            Some(bb_imager::Destination::SdCard { path, .. }),
        ) => {
            bb_imager::flasher::sd::LinuxSd::new(img, path, customization.into())
                .flash(Some(chan))
                .await
        }
        (
            Some(BoardImage::Image { img, .. }),
            FlashingCustomization::Bcf(customization),
            Some(bb_imager::Destination::Port(port)),
        ) => {
            bb_imager::flasher::bcf::BeagleConnectFreedom::new(img, port, customization.into())
                .flash(Some(chan))
                .await
        }
        (
            Some(BoardImage::Image { img, .. }),
            FlashingCustomization::Msp430,
            Some(bb_imager::Destination::HidRaw(port)),
        ) => {
            bb_imager::flasher::msp430::Msp430::new(img, port)
                .flash(Some(chan))
                .await
        }
        #[cfg(feature = "pb2_mspm0")]
        (Some(BoardImage::Image { img, .. }), FlashingCustomization::Pb2Mspm0(x), _) => {
            bb_imager::flasher::pb2_mspm0::Pb2Mspm0::new(img, x.persist_eeprom)
                .flash(Some(chan))
                .await
        }
        _ => unreachable!(),
    }
}
