use std::{
    borrow::Cow, collections::HashMap, fmt::Display, path::PathBuf, sync::LazyLock, time::Duration,
};

use crate::{BBImagerMessage, PACKAGE_QUALIFIER, constants};
use bb_config::config;
use bb_flasher::{BBFlasher, BBFlasherTarget, DownloadFlashingStatus, sd::FlashingSdLinuxConfig};
use iced::{futures, widget};
use url::Url;

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) enum BoardImageIcon {
    Remote(url::Url),
    Local,
    Format,
}

#[derive(Debug, Clone, serde::Serialize)]
#[allow(clippy::large_enum_variant)]
pub(crate) enum BoardImage {
    SdFormat {
        details: Vec<(&'static str, String)>,
    },
    Image {
        flasher: config::Flasher,
        init_format: config::InitFormat,
        img: SelectedImage,
        bmap: Option<Bmap>,
        info_text: Option<String>,
        description: Option<String>,
        icon: BoardImageIcon,
        details: Vec<(&'static str, String)>,
    },
}

impl BoardImage {
    pub(crate) fn local(path: PathBuf, flasher: config::Flasher) -> Self {
        let metadata = std::fs::metadata(&path).expect("File does not exist");
        let details = vec![
            ("Path", path.to_string_lossy().to_string()),
            ("Size", metadata.len().to_string()),
        ];

        Self::Image {
            img: bb_flasher::LocalImage::new(path.into()).into(),
            bmap: None,
            flasher,
            // Do not try to apply customization for local images
            init_format: config::InitFormat::None,
            info_text: None,
            description: None,
            icon: BoardImageIcon::Local,
            details,
        }
    }

    pub(crate) fn remote(
        image: crate::db::OsImage,
        flasher: config::Flasher,
        downloader: bb_downloader::Downloader,
    ) -> Self {
        let mut details = vec![
            ("Release Date", image.release_date.to_string()),
            ("Image Size", pretty_bytes(image.extract_size as u64)),
        ];

        if let Some(x) = image.image_download_size {
            details.push(("Download Size", pretty_bytes(x as u64)))
        }

        Self::Image {
            img: RemoteImage::new(
                image.name.into(),
                Box::new(image.url.into()),
                image.image_download_sha256,
                image.extract_size as u64,
                downloader.clone(),
            )
            .into(),
            bmap: image.bmap.map(|url| Bmap {
                url: Box::new(url.into()),
                downloader,
            }),
            flasher,
            init_format: image.init_format,
            info_text: image.info_text,
            description: Some(image.description),
            icon: BoardImageIcon::Remote(image.icon.into()),
            details,
        }
    }

    pub(crate) fn format() -> Self {
        Self::SdFormat {
            details: vec![("Format", "FAT32".to_string())],
        }
    }

    pub(crate) fn description(&self) -> Option<&str> {
        match self {
            BoardImage::SdFormat { .. } => Some("Format a SD Card to FAT32 for reuse."),
            BoardImage::Image { description, .. } => description.as_ref().map(|x| x.as_str()),
        }
    }

    pub(crate) fn icon(&self) -> &BoardImageIcon {
        match self {
            BoardImage::SdFormat { .. } => &BoardImageIcon::Format,
            BoardImage::Image { icon, .. } => icon,
        }
    }

    pub(crate) const fn flasher(&self) -> config::Flasher {
        match self {
            BoardImage::SdFormat { .. } => config::Flasher::SdCard,
            BoardImage::Image { flasher, .. } => *flasher,
        }
    }

    pub(crate) const fn init_format(&self) -> config::InitFormat {
        match self {
            BoardImage::Image { init_format, .. } => *init_format,
            BoardImage::SdFormat { .. } => config::InitFormat::None,
        }
    }

    pub(crate) fn info_text(&self) -> Option<&str> {
        match self {
            BoardImage::Image { info_text, .. } => info_text.as_ref().map(|x| x.as_str()),
            BoardImage::SdFormat { .. } => None,
        }
    }

    pub(crate) fn file_name(&self) -> Option<String> {
        match self {
            Self::SdFormat { .. } => None,
            Self::Image { img, .. } => Some(img.file_name()),
        }
    }

    pub(crate) fn details(&self) -> &[(&'static str, String)] {
        match self {
            BoardImage::SdFormat { details } => details,
            BoardImage::Image { details, .. } => details,
        }
    }
}

impl std::fmt::Display for BoardImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoardImage::SdFormat { .. } => write!(f, "Format SD Card"),
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
        let lang = whoami::lang_prefs().ok()?.message_langs().next()?;
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

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct RemoteImage {
    name: Box<str>,
    url: Box<url::Url>,
    #[serde(with = "const_hex")]
    extract_sha256: [u8; 32],
    extract_size: u64,
    #[serde(skip)]
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

impl IntoFuture for RemoteImage {
    type Output = std::io::Result<(bb_flasher::OsImage, u64)>;
    type IntoFuture = std::pin::Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
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
                let t: tokio::task::JoinHandle<std::io::Result<()>> = tokio::spawn(async move {
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
                    bb_flasher::OsImage::from_piped(rx, t, extract_size)
                })
                .await
                .unwrap()?;
                Ok((img, self.extract_size))
            }
        })
    }
}

impl std::fmt::Display for RemoteImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct Bmap {
    url: Box<Url>,
    #[serde(skip)]
    downloader: bb_downloader::Downloader,
}

impl IntoFuture for Bmap {
    type Output = std::io::Result<Box<str>>;
    type IntoFuture = std::pin::Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let p = self.downloader.download(*self.url.clone()).await?;
            tokio::fs::read_to_string(p).await.map(Into::into)
        })
    }
}

#[derive(Debug, Clone, serde::Serialize)]
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

impl IntoFuture for SelectedImage {
    type Output = std::io::Result<(bb_flasher::OsImage, u64)>;
    type IntoFuture = std::pin::Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        match self {
            SelectedImage::LocalImage(x) => x.into_future(),
            SelectedImage::RemoteImage(x) => x.into_future(),
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
    img: BoardImage,
    customization: FlashingCustomization,
    dst: Destination,
    chan: futures::channel::mpsc::Sender<DownloadFlashingStatus>,
    cancel: tokio_util::sync::CancellationToken,
) -> anyhow::Result<()> {
    match (img, customization, dst) {
        (BoardImage::Image { img, .. }, _, Destination::LocalFile(f)) => {
            img.save(&f, chan).await.map_err(Into::into)
        }
        (BoardImage::SdFormat { .. }, _, Destination::SdCard(t)) => {
            bb_flasher::sd::FormatFlasher::new(t)
                .flash(Some(chan))
                .await
        }
        (
            BoardImage::Image { img, bmap, .. },
            FlashingCustomization::LinuxSdSysconfig(customization),
            Destination::SdCard(t),
        ) => {
            bb_flasher::sd::Flasher::new(
                img.into_future(),
                bmap.map(IntoFuture::into_future),
                t,
                customization.into(),
                Some(cancel),
            )
            .flash(Some(chan))
            .await
        }
        (
            BoardImage::Image { img, bmap, .. },
            FlashingCustomization::NoneSd,
            Destination::SdCard(t),
        ) => {
            bb_flasher::sd::Flasher::new(
                img.into_future(),
                bmap.map(IntoFuture::into_future),
                t,
                FlashingSdLinuxConfig::none(),
                Some(cancel),
            )
            .flash(Some(chan))
            .await
        }
        #[cfg(feature = "bcf_cc1352p7")]
        (
            BoardImage::Image { img, .. },
            FlashingCustomization::Bcf(customization),
            Destination::BeagleConnectFreedom(t),
        ) => {
            bb_flasher::bcf::cc1352p7::Flasher::new(
                img.into_future(),
                t,
                customization.verify,
                Some(cancel),
            )
            .flash(Some(chan))
            .await
        }
        #[cfg(feature = "bcf_msp430")]
        (BoardImage::Image { img, .. }, FlashingCustomization::Msp430, Destination::Msp430(t)) => {
            bb_flasher::bcf::msp430::Flasher::new(img.into_future(), t)
                .flash(Some(chan))
                .await
        }
        #[cfg(feature = "pb2_mspm0")]
        (
            BoardImage::Image { img, .. },
            FlashingCustomization::Pb2Mspm0(x),
            Destination::Pb2Mspm0,
        ) => {
            bb_flasher::pb2::mspm0::Flasher::new(img.into_future(), x.persist_eeprom)
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
    Pb2Mspm0,
}

impl Display for Destination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Destination::LocalFile(_) => write!(f, "Save To File"),
            Destination::SdCard(target) => target.fmt(f),
            #[cfg(feature = "bcf_cc1352p7")]
            Destination::BeagleConnectFreedom(target) => target.fmt(f),
            #[cfg(feature = "bcf_msp430")]
            Destination::Msp430(target) => target.fmt(f),
            #[cfg(feature = "pb2_mspm0")]
            Destination::Pb2Mspm0 => write!(f, "PocketBeagle 2 MSPM0"),
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

    pub(crate) fn details(&self) -> Vec<(&'static str, String)> {
        match self {
            Self::LocalFile(p) => vec![("Path", p.to_string_lossy().to_string())],
            Self::SdCard(t) => vec![
                ("Path", t.path().to_string_lossy().to_string()),
                ("Size", pretty_bytes(t.size())),
            ],
            #[cfg(feature = "bcf_cc1352p7")]
            Self::BeagleConnectFreedom(t) => vec![("Path", t.path().to_string())],
            #[cfg(feature = "bcf_msp430")]
            Self::Msp430(t) => vec![("Path", t.path().to_string())],
            #[cfg(feature = "pb2_mspm0")]
            Self::Pb2Mspm0 => Vec::new(),
        }
    }
}

pub(crate) async fn destinations(flasher: config::Flasher, filter: bool) -> Vec<Destination> {
    match flasher {
        config::Flasher::SdCard => bb_flasher::sd::Target::destinations(filter)
            .await
            .into_iter()
            .map(Destination::SdCard)
            .collect(),
        #[cfg(feature = "bcf_cc1352p7")]
        config::Flasher::BeagleConnectFreedom => {
            bb_flasher::bcf::cc1352p7::Target::destinations(filter)
                .await
                .into_iter()
                .map(Destination::BeagleConnectFreedom)
                .collect()
        }
        #[cfg(feature = "bcf_msp430")]
        config::Flasher::Msp430Usb => bb_flasher::bcf::msp430::Target::destinations(filter)
            .await
            .into_iter()
            .map(Destination::Msp430)
            .collect(),
        #[cfg(feature = "pb2_mspm0")]
        config::Flasher::Pb2Mspm0 => vec![Destination::Pb2Mspm0],
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

pub(crate) const fn flasher_supported(flasher: config::Flasher) -> bool {
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

    pub(crate) fn reset(&mut self) {
        match self {
            Self::LinuxSdSysconfig(_) => {
                *self = Self::LinuxSdSysconfig(Default::default());
            }
            Self::Bcf(_) => {
                *self = Self::Bcf(Default::default());
            }
            #[cfg(feature = "pb2_mspm0")]
            Self::Pb2Mspm0(_) => {
                *self = Self::Pb2Mspm0(Default::default());
            }
            _ => {}
        };
    }

    pub(crate) fn validate(&self) -> bool {
        match self {
            FlashingCustomization::LinuxSdSysconfig(sd_customization) => {
                sd_customization.validate_user()
            }
            _ => true,
        }
    }
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
    let dirs = project_dirs().unwrap();
    dirs.cache_dir().with_file_name(format!(
        "{}.{}.{}.log",
        PACKAGE_QUALIFIER.0, PACKAGE_QUALIFIER.1, PACKAGE_QUALIFIER.2
    ))
}

#[derive(Default, Debug)]
pub(crate) struct ImageHandleCache(HashMap<url::Url, ImageHandleCacheValue>);

#[derive(Debug)]
pub(crate) enum ImageHandleCacheValue {
    Svg(widget::svg::Handle),
    Img(widget::image::Handle),
}

impl From<PathBuf> for ImageHandleCacheValue {
    fn from(value: PathBuf) -> Self {
        let img = std::fs::read(&value).expect("Failed to open image");
        match image::guess_format(&img) {
            Ok(_) => Self::Img(widget::image::Handle::from_path(value)),
            Err(_) => Self::Svg(widget::svg::Handle::from_memory(img)),
        }
    }
}

impl ImageHandleCacheValue {
    pub(crate) fn view<'a>(
        &'a self,
        width: impl Into<iced::Length>,
        height: impl Into<iced::Length>,
    ) -> iced::Element<'a, BBImagerMessage> {
        match self {
            ImageHandleCacheValue::Svg(handle) => widget::svg(handle.clone())
                .width(width)
                .height(height)
                .into(),
            ImageHandleCacheValue::Img(handle) => {
                widget::image(handle).width(width).height(height).into()
            }
        }
    }
}

impl ImageHandleCache {
    pub(crate) fn get(&self, u: &url::Url) -> Option<&ImageHandleCacheValue> {
        self.0.get(u)
    }

    pub(crate) fn insert(&mut self, u: url::Url, path: PathBuf) {
        self.0.insert(u, path.into());
    }
}

impl Extend<(url::Url, PathBuf)> for ImageHandleCache {
    fn extend<T: IntoIterator<Item = (url::Url, PathBuf)>>(&mut self, iter: T) {
        self.0.extend(iter.into_iter().map(|(k, p)| (k, p.into())))
    }
}

impl FromIterator<(url::Url, PathBuf)> for ImageHandleCache {
    fn from_iter<T: IntoIterator<Item = (url::Url, PathBuf)>>(iter: T) -> Self {
        Self(HashMap::from_iter(
            iter.into_iter().map(|(k, p)| (k, p.into())),
        ))
    }
}

pub(crate) fn pretty_bytes(bytes: u64) -> String {
    const UNITS: [&str; 7] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit = 0;

    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{:.2} {}", size, UNITS[unit])
    }
}

pub(crate) const fn static_destination(flasher: config::Flasher) -> Option<Destination> {
    match flasher {
        #[cfg(feature = "pb2_mspm0")]
        config::Flasher::Pb2Mspm0 => Some(Destination::Pb2Mspm0),
        _ => None,
    }
}

/// Return customization enum variant for cases where no customization is present
pub(crate) fn no_customization(
    flasher: config::Flasher,
    img: &BoardImage,
    dst: &Destination,
) -> Option<FlashingCustomization> {
    if dst.is_download_action() {
        return Some(FlashingCustomization::NoneSd);
    }

    match flasher {
        config::Flasher::SdCard if img.init_format() == config::InitFormat::Sysconf => None,
        config::Flasher::SdCard => Some(FlashingCustomization::NoneSd),
        config::Flasher::Msp430Usb => Some(FlashingCustomization::Msp430),
        _ => None,
    }
}

pub(crate) fn pretty_duration(d: Duration) -> String {
    let secs = d.as_secs();

    if secs >= 60 {
        format!("{}:{:02}", secs / 60, secs % 60)
    } else {
        format!("{}s", secs)
    }
}

pub(crate) fn app_title(_: &crate::BBImager) -> String {
    if option_env!("PRE_RELEASE").is_some() {
        format!("{} (pre-release)", constants::APP_NAME)
    } else {
        format!("{} v{}", constants::APP_NAME, env!("CARGO_PKG_VERSION"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OsImageId {
    Format,
    // points to parent
    Local(config::Flasher),
    // points to OsImage
    OsImage(i64),
    OsSublist(i64),
}

#[derive(Debug, Clone)]
pub(crate) struct OsImageItem {
    pub(crate) id: OsImageId,
    pub(crate) icon: Option<url::Url>,
    pub(crate) label: Cow<'static, str>,
}

impl From<crate::db::OsImageListItem> for OsImageItem {
    fn from(value: crate::db::OsImageListItem) -> Self {
        Self {
            id: OsImageId::OsImage(value.id),
            icon: Some(value.icon.into()),
            label: Cow::Owned(value.name),
        }
    }
}

impl From<crate::db::OsSublistListItem> for OsImageItem {
    fn from(value: crate::db::OsSublistListItem) -> Self {
        Self {
            id: OsImageId::OsSublist(value.id),
            icon: Some(value.icon.into()),
            label: Cow::Owned(value.name),
        }
    }
}

impl OsImageItem {
    pub(crate) fn format(label: Cow<'static, str>) -> Self {
        Self {
            id: OsImageId::Format,
            icon: None,
            label,
        }
    }

    pub(crate) fn local(flasher: config::Flasher) -> Self {
        Self {
            id: OsImageId::Local(flasher),
            icon: None,
            label: Cow::Borrowed("Select Local Image"),
        }
    }

    pub(crate) const fn is_sublist(&self) -> bool {
        matches!(self.id, OsImageId::OsSublist(_))
    }

    pub(crate) fn label(&self) -> &str {
        &self.label
    }
}

#[derive(Debug)]
pub(crate) enum DestinationItem<'a> {
    SaveToFile(String),
    Destination(&'a Destination),
}

impl<'a> std::fmt::Display for DestinationItem<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DestinationItem::SaveToFile(_) => write!(f, "Save To File"),
            DestinationItem::Destination(d) => d.fmt(f),
        }
    }
}

impl<'a> DestinationItem<'a> {
    pub(crate) fn msg(&'a self) -> BBImagerMessage {
        match self {
            DestinationItem::SaveToFile(x) => BBImagerMessage::SelectFileDest(x.clone()),
            DestinationItem::Destination(d) => BBImagerMessage::SelectDest((*d).clone()),
        }
    }

    pub(crate) fn is_selected(&'a self, dst: &'a Destination) -> bool {
        match self {
            DestinationItem::SaveToFile(_) => false,
            DestinationItem::Destination(d) => dst.eq(d),
        }
    }
}

pub(crate) fn fetch_images(
    downloader: &bb_downloader::Downloader,
    iter: impl IntoIterator<Item = url::Url>,
) -> iced::Task<BBImagerMessage> {
    let tasks = iter.into_iter().map(|icon| {
        let downloader = downloader.clone();
        let icon_clone = icon.clone();
        let icon_clone2 = icon.clone();
        iced::Task::perform(
            async move { downloader.download_no_cache(icon_clone).await },
            move |p| match p {
                Ok(p) => BBImagerMessage::ResolveImage(icon_clone2, p),
                Err(_) => {
                    tracing::warn!("Failed to fetch image {}", icon);
                    BBImagerMessage::Null
                }
            },
        )
    });

    iced::Task::batch(tasks)
}
