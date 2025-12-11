use std::{borrow::Cow, fmt::Display, path::PathBuf, sync::LazyLock};

use crate::{BBImagerMessage, PACKAGE_QUALIFIER};
use bb_config::config::{self, OsListItem};
use bb_flasher::{BBFlasher, BBFlasherTarget, DownloadFlashingStatus, sd::FlashingSdLinuxConfig};
use iced::{
    Color, Length, futures,
    widget::{self, Column, progress_bar, text},
};
use iced_loading::Linear;
use url::Url;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ProgressBarState {
    label: Cow<'static, str>,
    progress: f32,
    state: ProgressBarStatus,
    inner_state: Option<DownloadFlashingStatus>,
}

impl ProgressBarState {
    pub(crate) const FLASHING_SUCCESS: Self =
        Self::const_new("Flashing Successful", 1.0, ProgressBarStatus::Success, None);
    pub(crate) const PREPARING: Self =
        Self::loading("Preparing...", DownloadFlashingStatus::Preparing);
    pub(crate) const VERIFYING: Self =
        Self::loading("Verifying...", DownloadFlashingStatus::Verifying);
    pub(crate) const CUSTOMIZING: Self =
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

    pub(crate) fn content(&self) -> String {
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

    pub(crate) fn fail(label: impl Into<Cow<'static, str>>) -> Self {
        Self::new(label, 1.0, ProgressBarStatus::Fail, None)
    }

    pub(crate) fn bar(&self) -> Column<'_, BBImagerMessage> {
        use std::ops::RangeInclusive;

        const RANGE: RangeInclusive<f32> = (0.0)..=1.0;

        if self.state == ProgressBarStatus::Loading {
            widget::column![
                text(self.label.clone()).color(Color::WHITE),
                Linear::new()
                    .width(Length::Fill)
                    .height(8.0)
                    .cycle_duration(std::time::Duration::from_millis(1000))
                    // TODO: Add back color
                    // .color(Color::from_rgb(0.0, 0.5, 1.0)),
            ]
        } else {
            widget::column![
                text(self.label.clone()).color(Color::WHITE),
                progress_bar(RANGE, self.progress)
                    .girth(8)
                    .style(self.state.style()),
            ]
        }
        .align_x(iced::Alignment::Center)
        .spacing(12)
    }

    pub(crate) fn cancel(&self) -> Option<Self> {
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
pub(crate) enum ProgressBarStatus {
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
pub(crate) struct Boards {
    config: config::Config,
}

impl Boards {
    pub(crate) fn merge(&mut self, config: bb_config::Config) {
        self.config.extend([config])
    }

    pub(crate) fn unrsolved_configs(&self) -> impl Iterator<Item = &Url> {
        self.config.imager.remote_configs.iter()
    }

    pub(crate) fn devices(&self) -> impl Iterator<Item = (usize, &config::Device)> {
        self.config.imager.devices.iter().enumerate()
    }

    pub(crate) fn image(&self, target: &[usize]) -> &OsListItem {
        let mut res = &self.config.os_list;
        let (last, rest) = target.split_last().unwrap();

        for i in rest {
            let item = res.get(*i).expect("No Subitem");
            res = match item {
                OsListItem::Image(_) => panic!("No subitem"),
                OsListItem::SubList(item) => &item.subitems,
                OsListItem::RemoteSubList { .. } => panic!("No subitem"),
            }
        }

        res.get(*last).unwrap()
    }

    pub(crate) fn images(
        &self,
        board_idx: usize,
        subitems: &[usize],
    ) -> Option<Vec<(usize, &OsListItem)>> {
        let mut res = &self.config.os_list;

        for i in subitems {
            let item = res.get(*i).expect("No Subitem");
            res = match item {
                OsListItem::Image(_) => panic!("No subitem"),
                OsListItem::SubList(item) => &item.subitems,
                OsListItem::RemoteSubList { .. } => return None,
            }
        }

        let dev = self.device(board_idx);
        let tags = &dev.tags;

        Some(
            res.iter()
                .enumerate()
                .filter(move |(_, x)| x.has_board_image(tags))
                .filter(|(_, x)| match x {
                    OsListItem::RemoteSubList(item) => flasher_supported(item.flasher),
                    OsListItem::SubList(item) => flasher_supported(item.flasher),
                    _ => true,
                })
                .collect(),
        )
    }

    pub(crate) fn device(&self, board_idx: usize) -> &config::Device {
        self.config
            .imager
            .devices
            .get(board_idx)
            .expect("Board does not exist")
    }

    pub(crate) fn resolve_remote_subitem(&mut self, subitems: Vec<OsListItem>, target: &[usize]) {
        assert!(!target.is_empty());

        let mut res = &mut self.config.os_list;

        let (last, rest) = target.split_last().unwrap();

        for i in rest {
            let item = res.get_mut(*i).expect("No Subitem");
            res = match item {
                OsListItem::Image(_) => panic!("No subitem"),
                OsListItem::SubList(item) => &mut item.subitems,
                OsListItem::RemoteSubList { .. } => panic!("No subitem"),
            }
        }

        if let OsListItem::RemoteSubList(item) = res.get(*last).unwrap().clone() {
            res[*last] = OsListItem::SubList(item.resolve(subitems))
        } else {
            tracing::warn!("Unexpected item")
        }
    }
}

impl From<config::Config> for Boards {
    fn from(value: config::Config) -> Self {
        let filtered = config::Config {
            imager: config::Imager {
                remote_configs: value.imager.remote_configs,
                devices: value
                    .imager
                    .devices
                    .into_iter()
                    .filter(|x| flasher_supported(x.flasher))
                    .collect(),
            },
            os_list: value.os_list,
        };
        Self { config: filtered }
    }
}

impl Default for Boards {
    fn default() -> Self {
        serde_json::from_slice::<config::Config>(crate::constants::DEFAULT_CONFIG)
            .expect("Failed to parse config")
            .into()
    }
}

#[derive(Debug, Clone)]
pub(crate) enum BoardImage {
    SdFormat,
    Image {
        flasher: config::Flasher,
        init_format: config::InitFormat,
        img: SelectedImage,
        bmap: Option<Bmap>,
        info_text: Option<String>,
    },
}

impl BoardImage {
    pub(crate) fn local(path: PathBuf, flasher: config::Flasher) -> Self {
        Self::Image {
            img: bb_flasher::LocalImage::new(path.into()).into(),
            bmap: None,
            flasher,
            // Do not try to apply customization for local images
            init_format: config::InitFormat::None,
            info_text: None,
        }
    }

    pub(crate) fn remote(
        image: config::OsImage,
        flasher: config::Flasher,
        downloader: bb_downloader::Downloader,
    ) -> Self {
        Self::Image {
            img: RemoteImage::new(
                image.name.into(),
                Box::new(image.url),
                image.image_download_sha256,
                image.extract_size,
                downloader.clone(),
            )
            .into(),
            bmap: image.bmap.map(|url| Bmap {
                url: Box::new(url),
                downloader,
            }),
            flasher,
            init_format: image.init_format,
            info_text: image.info_text,
        }
    }

    pub(crate) const fn flasher(&self) -> config::Flasher {
        match self {
            BoardImage::SdFormat => config::Flasher::SdCard,
            BoardImage::Image { flasher, .. } => *flasher,
        }
    }

    pub(crate) const fn init_format(&self) -> config::InitFormat {
        match self {
            BoardImage::Image { init_format, .. } => *init_format,
            BoardImage::SdFormat => config::InitFormat::None,
        }
    }

    pub(crate) fn info_text(&self) -> Option<String> {
        match self {
            BoardImage::Image { info_text, .. } => info_text.clone(),
            BoardImage::SdFormat => None,
        }
    }

    pub(crate) fn file_name(&self) -> Option<String> {
        match self {
            Self::SdFormat => None,
            Self::Image { img, .. } => Some(img.file_name()),
        }
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

pub(crate) fn system_timezone() -> Option<&'static String> {
    static SYSTEM_TIMEZONE: LazyLock<Option<String>> = LazyLock::new(localzone::get_local_zone);
    (*SYSTEM_TIMEZONE).as_ref()
}

pub(crate) fn system_keymap() -> String {
    static SYSTEM_KEYMAP: LazyLock<Option<String>> = LazyLock::new(|| {
        let lang = whoami::langs().ok()?.next()?;
        let lang_str = lang.to_string();

        let base = lang_str.split('.').next().unwrap_or(&lang_str);
        let mut parts = base.split(['-', '_', '/']);

        parts.next();
        if let Some(region) = parts.next() {
            let region = region.split('@').next().unwrap_or(region).trim();
            if !region.is_empty()
                && let Some(&canon) = crate::constants::KEYMAP_LAYOUTS
                    .iter()
                    .find(|k| k.eq_ignore_ascii_case(region))
            {
                return Some(canon.to_string());
            }
        }

        None
    });
    (*SYSTEM_KEYMAP)
        .as_ref()
        .cloned()
        .unwrap_or_else(|| String::from("us"))
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteImage {
    name: Box<str>,
    url: Box<url::Url>,
    extract_sha256: [u8; 32],
    extract_size: u64,
    downloader: bb_downloader::Downloader,
}

impl RemoteImage {
    pub(crate) fn new(
        name: Box<str>,
        url: Box<url::Url>,
        extract_sha256: [u8; 32],
        extract_size: u64,
        downloader: bb_downloader::Downloader,
    ) -> Self {
        Self {
            name,
            url,
            extract_sha256,
            extract_size,
            downloader,
        }
    }

    fn file_name(&self) -> &str {
        self.url.path_segments().unwrap().next_back().unwrap()
    }

    async fn save(
        &self,
        path: &std::path::Path,
        mut chan: futures::channel::mpsc::Sender<DownloadFlashingStatus>,
    ) -> std::io::Result<()> {
        let (tx, mut rx) = futures::channel::mpsc::channel(5);

        let handle = tokio::spawn(async move {
            while let Some(x) = futures::StreamExt::next(&mut rx).await {
                let _ = chan.try_send(DownloadFlashingStatus::DownloadingProgress(x));
            }
        });

        let p = self
            .downloader
            .download_with_sha(*self.url.clone(), self.extract_sha256, Some(tx))
            .await?;
        tokio::fs::copy(p, path).await?;
        handle.abort();

        Ok(())
    }
}

impl bb_flasher::Resolvable for RemoteImage {
    type ResolvedType = (bb_flasher::OsImage, u64);

    async fn resolve(
        &self,
        rt: &mut tokio::task::JoinSet<std::io::Result<()>>,
    ) -> std::io::Result<Self::ResolvedType> {
        if let Some(path) = self
            .downloader
            .check_cache_from_sha(self.extract_sha256)
            .await
        {
            tracing::info!("Found the remote image in cache");
            Ok((bb_flasher::OsImage::from_path(&path)?, self.extract_size))
        } else {
            tracing::info!("Remote image not found in cache. Downloading");
            let (tx, rx) = bb_helper::file_stream::file_stream()?;
            let downloader = self.downloader.clone();
            let url = self.url.clone();
            let sha = self.extract_sha256;
            rt.spawn(async move {
                downloader
                    .download_to_stream(*url, sha, tx)
                    .await
                    .map_err(|e| {
                        let msg = format!("Error while downloading Os Image: {e}");
                        tracing::error!("{}", &msg);
                        std::io::Error::other(msg)
                    })?;
                tracing::info!("Image download finished");
                Ok(())
            });

            let extract_size = self.extract_size;
            let img = tokio::task::spawn_blocking(move || {
                bb_flasher::OsImage::from_piped(rx, extract_size)
            })
            .await
            .unwrap()?;
            Ok((img, self.extract_size))
        }
    }
}

impl std::fmt::Display for RemoteImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Bmap {
    url: Box<Url>,
    downloader: bb_downloader::Downloader,
}

impl bb_flasher::Resolvable for Bmap {
    type ResolvedType = Box<str>;

    async fn resolve(
        &self,
        _: &mut tokio::task::JoinSet<std::io::Result<()>>,
    ) -> std::io::Result<Self::ResolvedType> {
        let p = self.downloader.download(*self.url.clone(), None).await?;

        tokio::fs::read_to_string(p).await.map(Into::into)
    }
}

#[derive(Debug, Clone)]
pub(crate) enum SelectedImage {
    LocalImage(bb_flasher::LocalImage),
    RemoteImage(RemoteImage),
}

impl SelectedImage {
    fn file_name(&self) -> String {
        match self {
            Self::LocalImage(x) => x.file_name().to_string_lossy().to_string(),
            Self::RemoteImage(x) => x.file_name().to_string(),
        }
    }

    async fn save(
        &self,
        path: &std::path::Path,
        chan: futures::channel::mpsc::Sender<DownloadFlashingStatus>,
    ) -> std::io::Result<()> {
        match self {
            Self::LocalImage(x) => tokio::fs::copy(x.path(), path).await.map(|_| ()),
            Self::RemoteImage(x) => x.save(path, chan).await,
        }
    }
}

impl bb_flasher::Resolvable for SelectedImage {
    type ResolvedType = (bb_flasher::OsImage, u64);

    async fn resolve(
        &self,
        rt: &mut tokio::task::JoinSet<std::io::Result<()>>,
    ) -> std::io::Result<Self::ResolvedType> {
        match self {
            SelectedImage::LocalImage(x) => x.resolve(rt).await,
            SelectedImage::RemoteImage(x) => x.resolve(rt).await,
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

impl From<bb_flasher::LocalImage> for SelectedImage {
    fn from(value: bb_flasher::LocalImage) -> Self {
        Self::LocalImage(value)
    }
}

pub(crate) async fn flash(
    img: Option<BoardImage>,
    customization: FlashingCustomization,
    dst: Option<Destination>,
    chan: futures::channel::mpsc::Sender<DownloadFlashingStatus>,
    cancel: tokio_util::sync::CancellationToken,
) -> anyhow::Result<()> {
    match (img, customization, dst) {
        (Some(BoardImage::Image { img, .. }), _, Some(Destination::LocalFile(f))) => {
            img.save(&f, chan).await.map_err(Into::into)
        }
        (Some(BoardImage::SdFormat), _, Some(Destination::SdCard(t))) => {
            bb_flasher::sd::FormatFlasher::new(t)
                .flash(Some(chan))
                .await
        }
        (
            Some(BoardImage::Image { img, bmap, .. }),
            FlashingCustomization::LinuxSdSysconfig(customization),
            Some(Destination::SdCard(t)),
        ) => {
            bb_flasher::sd::Flasher::new(img, bmap, t, customization.into(), Some(cancel))
                .flash(Some(chan))
                .await
        }
        (
            Some(BoardImage::Image { img, bmap, .. }),
            FlashingCustomization::NoneSd,
            Some(Destination::SdCard(t)),
        ) => {
            bb_flasher::sd::Flasher::new(img, bmap, t, FlashingSdLinuxConfig::none(), Some(cancel))
                .flash(Some(chan))
                .await
        }
        #[cfg(feature = "bcf_cc1352p7")]
        (
            Some(BoardImage::Image { img, .. }),
            FlashingCustomization::Bcf(customization),
            Some(Destination::BeagleConnectFreedom(t)),
        ) => {
            bb_flasher::bcf::cc1352p7::Flasher::new(img, t, customization.verify, Some(cancel))
                .flash(Some(chan))
                .await
        }
        #[cfg(feature = "bcf_msp430")]
        (
            Some(BoardImage::Image { img, .. }),
            FlashingCustomization::Msp430,
            Some(Destination::Msp430(t)),
        ) => {
            bb_flasher::bcf::msp430::Flasher::new(img, t)
                .flash(Some(chan))
                .await
        }
        #[cfg(feature = "pb2_mspm0")]
        (Some(BoardImage::Image { img, .. }), FlashingCustomization::Pb2Mspm0(x), _) => {
            bb_flasher::pb2::mspm0::Flasher::new(img, x.persist_eeprom)
                .flash(Some(chan))
                .await
        }
        _ => unimplemented!(),
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) enum Destination {
    LocalFile(PathBuf),
    SdCard(bb_flasher::sd::Target),
    #[cfg(feature = "bcf_cc1352p7")]
    BeagleConnectFreedom(bb_flasher::bcf::cc1352p7::Target),
    #[cfg(feature = "bcf_msp430")]
    Msp430(bb_flasher::bcf::msp430::Target),
    #[cfg(feature = "pb2_mspm0")]
    Pb2Mspm0(bb_flasher::pb2::mspm0::Target),
}

impl Display for Destination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Destination::LocalFile(x) => x.display().fmt(f),
            Destination::SdCard(target) => target.fmt(f),
            #[cfg(feature = "bcf_cc1352p7")]
            Destination::BeagleConnectFreedom(target) => target.fmt(f),
            #[cfg(feature = "bcf_msp430")]
            Destination::Msp430(target) => target.fmt(f),
            #[cfg(feature = "pb2_mspm0")]
            Destination::Pb2Mspm0(target) => target.fmt(f),
        }
    }
}

impl Destination {
    #[allow(irrefutable_let_patterns)]
    pub(crate) fn size(&self) -> Option<u64> {
        if let Destination::SdCard(item) = self {
            Some(item.size())
        } else {
            None
        }
    }

    /// Download instead of flashing
    pub(crate) fn is_download_action(&self) -> bool {
        matches!(self, Self::LocalFile(_))
    }
}

pub(crate) async fn destinations(flasher: config::Flasher) -> Vec<Destination> {
    match flasher {
        config::Flasher::SdCard => bb_flasher::sd::Target::destinations()
            .await
            .into_iter()
            .map(Destination::SdCard)
            .collect(),
        #[cfg(feature = "bcf_cc1352p7")]
        config::Flasher::BeagleConnectFreedom => bb_flasher::bcf::cc1352p7::Target::destinations()
            .await
            .into_iter()
            .map(Destination::BeagleConnectFreedom)
            .collect(),
        #[cfg(feature = "bcf_msp430")]
        config::Flasher::Msp430Usb => bb_flasher::bcf::msp430::Target::destinations()
            .await
            .into_iter()
            .map(Destination::Msp430)
            .collect(),
        #[cfg(feature = "pb2_mspm0")]
        config::Flasher::Pb2Mspm0 => bb_flasher::pb2::mspm0::Target::destinations()
            .await
            .into_iter()
            .map(Destination::Pb2Mspm0)
            .collect(),
        _ => unimplemented!(),
    }
}

pub(crate) fn is_destination_selectable(flasher: config::Flasher) -> bool {
    match flasher {
        config::Flasher::SdCard => bb_flasher::sd::Target::is_destination_selectable(),
        #[cfg(feature = "bcf_cc1352p7")]
        config::Flasher::BeagleConnectFreedom => {
            bb_flasher::bcf::cc1352p7::Target::is_destination_selectable()
        }
        #[cfg(feature = "bcf_msp430")]
        config::Flasher::Msp430Usb => bb_flasher::bcf::msp430::Target::is_destination_selectable(),
        #[cfg(feature = "pb2_mspm0")]
        config::Flasher::Pb2Mspm0 => bb_flasher::pb2::mspm0::Target::is_destination_selectable(),
        _ => unimplemented!(),
    }
}

pub(crate) fn file_filter(flasher: config::Flasher) -> &'static [&'static str] {
    match flasher {
        config::Flasher::SdCard => bb_flasher::sd::Target::FILE_TYPES,
        #[cfg(feature = "bcf_cc1352p7")]
        config::Flasher::BeagleConnectFreedom => bb_flasher::bcf::cc1352p7::Target::FILE_TYPES,
        #[cfg(feature = "bcf_msp430")]
        config::Flasher::Msp430Usb => bb_flasher::bcf::msp430::Target::FILE_TYPES,
        #[cfg(feature = "pb2_mspm0")]
        config::Flasher::Pb2Mspm0 => bb_flasher::pb2::mspm0::Target::FILE_TYPES,
        _ => unimplemented!(),
    }
}

const fn flasher_supported(flasher: config::Flasher) -> bool {
    match flasher {
        config::Flasher::SdCard => true,
        #[cfg(feature = "bcf_cc1352p7")]
        config::Flasher::BeagleConnectFreedom => true,
        #[cfg(feature = "bcf_msp430")]
        config::Flasher::Msp430Usb => true,
        #[cfg(feature = "pb2_mspm0")]
        config::Flasher::Pb2Mspm0 => true,
        _ => false,
    }
}

#[derive(Clone, Debug)]
pub(crate) enum FlashingCustomization {
    NoneSd,
    LinuxSdSysconfig(crate::persistance::SdSysconfCustomization),
    Bcf(crate::persistance::BcfCustomization),
    Msp430,
    #[cfg(feature = "pb2_mspm0")]
    Pb2Mspm0(crate::persistance::Pb2Mspm0Customization),
}

impl FlashingCustomization {
    pub(crate) fn new(
        flasher: config::Flasher,
        img: &BoardImage,
        app_config: &crate::persistance::GuiConfiguration,
    ) -> Self {
        match flasher {
            config::Flasher::SdCard if img.init_format() == config::InitFormat::Sysconf => {
                Self::LinuxSdSysconfig(
                    app_config
                        .sd_customization()
                        .map(|x| x.sysconf_customization().cloned().unwrap_or_default())
                        .unwrap_or_default(),
                )
            }
            config::Flasher::SdCard => Self::NoneSd,
            config::Flasher::BeagleConnectFreedom => {
                Self::Bcf(app_config.bcf_customization().cloned().unwrap_or_default())
            }
            config::Flasher::Msp430Usb => Self::Msp430,
            #[cfg(feature = "pb2_mspm0")]
            config::Flasher::Pb2Mspm0 => Self::Pb2Mspm0(
                app_config
                    .pb2_mspm0_customization()
                    .cloned()
                    .unwrap_or_default(),
            ),
            _ => unimplemented!(),
        }
    }

    pub(crate) fn reset(self) -> Self {
        match self {
            Self::LinuxSdSysconfig(_) => Self::LinuxSdSysconfig(Default::default()),
            Self::Bcf(_) => Self::Bcf(Default::default()),
            #[cfg(feature = "pb2_mspm0")]
            Self::Pb2Mspm0(_) => Self::Pb2Mspm0(Default::default()),
            _ => self,
        }
    }

    pub(crate) fn validate(&self) -> bool {
        match self {
            FlashingCustomization::LinuxSdSysconfig(sd_customization) => {
                sd_customization.validate_user()
            }
            _ => true,
        }
    }

    /// Check if any configuration is even present
    pub(crate) const fn need_confirmation(&self) -> bool {
        match self {
            Self::LinuxSdSysconfig(_) | Self::Bcf(_) => true,
            #[cfg(feature = "pb2_mspm0")]
            Self::Pb2Mspm0(_) => true,
            _ => false,
        }
    }
}

/// Fetches the main remote os_list file from `bb_config::DISTROS_URL` and merges it with the base
/// config.
async fn fetch_remote_os_list(
    client: bb_downloader::Downloader,
    url: Url,
) -> std::io::Result<config::Config> {
    client.download_json_no_cache(url).await
}

pub(crate) fn refresh_config_task(
    client: bb_downloader::Downloader,
    config: &Boards,
) -> iced::Task<BBImagerMessage> {
    let tasks = config.unrsolved_configs().map(|x| {
        iced::Task::perform(
            fetch_remote_os_list(client.clone(), x.clone()),
            |x: std::io::Result<config::Config>| match x {
                Ok(y) => BBImagerMessage::ExtendConfig(y),
                Err(e) => {
                    tracing::error!("Failed to fetch config: {e}");
                    BBImagerMessage::Null
                }
            },
        )
    });
    iced::Task::batch(tasks)
}

#[cfg(target_os = "linux")]
async fn show_notification_xdg_portal(body: &str) -> ashpd::Result<()> {
    let proxy = ashpd::desktop::notification::NotificationProxy::new().await?;

    let app_id = "org.beagleboard.imagingutility";
    proxy
        .add_notification(
            app_id,
            ashpd::desktop::notification::Notification::new("BeagleBoard Imager").body(body),
        )
        .await
}

pub(crate) async fn show_notification(body: String) -> notify_rust::error::Result<()> {
    #[cfg(target_os = "linux")]
    if show_notification_xdg_portal(&body).await.is_ok() {
        return Ok(());
    }

    tokio::task::spawn_blocking(move || {
        notify_rust::Notification::new()
            .appname("BeagleBoard Imager")
            .body(&body)
            .finalize()
            .show()
    })
    .await
    .unwrap()
    .map(|_| ())
}

pub(crate) fn project_dirs() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from(
        crate::constants::PACKAGE_QUALIFIER.0,
        crate::constants::PACKAGE_QUALIFIER.1,
        crate::constants::PACKAGE_QUALIFIER.2,
    )
}

pub(crate) fn log_file_path() -> PathBuf {
    let dirs = crate::helpers::project_dirs().unwrap();
    dirs.cache_dir().with_file_name(format!(
        "{}.{}.{}.log",
        PACKAGE_QUALIFIER.0, PACKAGE_QUALIFIER.1, PACKAGE_QUALIFIER.2
    ))
}
