use std::io;
use std::{fmt::Display, path::PathBuf, sync::LazyLock};

use crate::{BBImagerMessage, PACKAGE_QUALIFIER, constants};
use bb_config::config;
#[allow(unused)]
use bb_flasher::BBFlasherTarget as _;
use bb_flasher::DownloadFlashingStatus;
#[cfg(feature = "sd")]
use bb_flasher::img::OsArchive;
use bb_flasher::img::OsImage;
use bb_helper::file_stream::ReaderFileStream;
#[allow(unused)]
use bb_imager_ui::dest_selection::Destination as _;
use bb_imager_ui::{Message, customization};
use std::sync::{Arc, mpsc};
use tokio_util::task::AbortOnDropHandle;
use url::Url;

#[derive(serde::Serialize)]
pub(crate) enum ImageInfo {
    Format,
    Local(Box<std::path::Path>),
    Remote(Box<config::OsImage>),
}

#[derive(serde::Serialize)]
pub(crate) struct FlashingInfo {
    pub(crate) board: config::Device,
    pub(crate) image: ImageInfo,
    pub(crate) destination: Box<str>,
    pub(crate) customization: FlashingCustomization,
}

impl FlashingInfo {
    pub(crate) fn json(
        db: crate::db::Db,
        ctx: &crate::state::FlashingContext,
    ) -> iced::Task<String> {
        let board_id = ctx.selected_board.id;
        let img = ctx.selected_image.clone();
        let customization = ctx.customization.clone();
        let destination = ctx.selected_dest.to_string().into();

        iced::Task::perform(
            async move {
                let board = db.os_board_json_by_id(board_id).unwrap();

                let image = match img {
                    BoardImage::SdFormat => ImageInfo::Format,
                    BoardImage::Image { img, .. } => match img {
                        SelectedImage::LocalImage(p) => ImageInfo::Local(p.path().into()),
                        SelectedImage::RemoteImage(x) => {
                            ImageInfo::Remote(db.os_image_json_by_id(x.id).unwrap().into())
                        }
                    },
                };

                FlashingInfo {
                    board,
                    customization,
                    image,
                    destination,
                }
            },
            |x| serde_json::to_string_pretty(&x).unwrap(),
        )
    }
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub(crate) enum BoardImage {
    SdFormat,
    Image {
        flasher: config::Flasher,
        init_format: config::InitFormat,
        img: SelectedImage,
        #[cfg(feature = "sd")]
        bmap: Option<Bmap>,
        info_text: Option<Arc<str>>,
    },
}

impl BoardImage {
    pub(crate) fn local(path: PathBuf, flasher: config::Flasher) -> Self {
        Self::Image {
            img: bb_flasher::LocalImage::new(path.into()).into(),
            #[cfg(feature = "sd")]
            bmap: None,
            flasher,
            // Do not try to apply customization for local images
            init_format: config::InitFormat::None,
            info_text: None,
        }
    }

    pub(crate) fn remote(
        image: crate::db::OsImage,
        flasher: config::Flasher,
        downloader: bb_downloader::Downloader,
    ) -> Self {
        Self::Image {
            img: RemoteImage::new(
                image.id,
                image.name,
                image.url,
                image.image_download_sha256,
                image.extract_size as u64,
                downloader.clone(),
            )
            .into(),
            #[cfg(feature = "sd")]
            bmap: image.bmap.map(|url| Bmap { url, downloader }),
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

    pub(crate) fn info_text(&self) -> Option<&str> {
        match self {
            BoardImage::Image { info_text, .. } => info_text.as_ref().map(|x| x.as_ref()),
            BoardImage::SdFormat => None,
        }
    }

    pub(crate) fn file_name(&self) -> Option<String> {
        match self {
            Self::SdFormat { .. } => None,
            Self::Image { img, .. } => Some(img.file_name()),
        }
    }

    pub(crate) fn is_local(&self) -> bool {
        match self {
            BoardImage::SdFormat => false,
            BoardImage::Image { img, .. } => matches!(img, SelectedImage::LocalImage(_)),
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

pub(crate) fn system_timezone() -> Option<chrono_tz::Tz> {
    static SYSTEM_TIMEZONE: LazyLock<Option<chrono_tz::Tz>> =
        LazyLock::new(|| iana_time_zone::get_timezone().ok()?.parse().ok());
    *SYSTEM_TIMEZONE
}

pub(crate) fn system_keymap() -> &'static str {
    static SYSTEM_KEYMAP: LazyLock<Option<&'static str>> = LazyLock::new(|| {
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
                return Some(canon);
            }
        }

        None
    });
    (*SYSTEM_KEYMAP).unwrap_or("us")
}

/// Username to pre-fill the customization page with.
///
/// Falls back to "beagle" rather than an empty name: an empty username is not a
/// valid account, and nothing downstream rejects one, so it would be written to
/// the image as-is.
pub(crate) fn default_user() -> &'static str {
    static USER: LazyLock<Option<String>> = LazyLock::new(|| whoami::username().ok());

    match &*USER {
        Some(x) => x,
        None => "beagle",
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteImage {
    pub(crate) id: i64,
    name: Box<str>,
    url: Box<url::Url>,
    extract_sha256: [u8; 32],
    extract_size: u64,
    downloader: bb_downloader::Downloader,
}

impl RemoteImage {
    pub(crate) fn new(
        id: i64,
        name: Box<str>,
        url: Box<url::Url>,
        extract_sha256: [u8; 32],
        extract_size: u64,
        downloader: bb_downloader::Downloader,
    ) -> Self {
        Self {
            id,
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

    fn open<C, P, R>(self, f_cache: C, f_pipe: P) -> impl FnOnce() -> io::Result<R>
    where
        C: FnOnce(&std::path::Path) -> io::Result<R>,
        P: FnOnce(ReaderFileStream, AbortOnDropHandle<io::Result<()>>, u64) -> io::Result<R>,
    {
        let rt = tokio::runtime::Handle::current();

        move || {
            let cache = self.downloader.check_cache_from_sha(self.extract_sha256);

            if let Some(path) = cache {
                tracing::info!("Found the remote image in cache");
                return f_cache(&path);
            }

            tracing::info!("Remote image not found in cache. Downloading");
            let (tx_stream, rx) = bb_helper::file_stream::file_stream()?;
            let sha = self.extract_sha256;

            let t: tokio::task::JoinHandle<io::Result<()>> = rt.spawn(async move {
                self.downloader
                    .download_to_stream(*self.url, sha, tx_stream)
                    .await
                    .map_err(|e| {
                        let msg = format!("Error while downloading Os Image: {e}");
                        tracing::error!("{}", &msg);
                        io::Error::other(msg)
                    })?;
                tracing::info!("Image download finished");
                Ok(())
            });

            f_pipe(rx, AbortOnDropHandle::new(t), self.extract_size)
        }
    }

    #[cfg(feature = "sd")]
    fn into_archive_fn(
        self,
        tx: Option<mpsc::SyncSender<f32>>,
    ) -> impl FnOnce() -> io::Result<OsArchive> {
        let tx_clone = tx.clone();
        self.open(
            move |p| OsArchive::from_path(p, tx_clone),
            move |rx, abort, es| OsArchive::from_piped(rx, abort, es, tx),
        )
    }

    fn into_image_fn(self) -> impl FnOnce() -> io::Result<(OsImage, u64)> {
        let extract_size = self.extract_size;
        self.open(
            move |p| Ok((OsImage::from_path(p)?, extract_size)),
            move |rx, abort, es| {
                let img = OsImage::from_piped(rx, abort, es)?;
                Ok((img, es))
            },
        )
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

impl Bmap {
    fn into_fn(self) -> impl FnOnce() -> io::Result<Box<str>> {
        let rt = tokio::runtime::Handle::current();
        move || {
            let res = rt.block_on(async move { self.downloader.download(*self.url).await })?;
            std::fs::read_to_string(res).map(Into::into)
        }
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

    #[cfg(feature = "sd")]
    fn into_archive_fn(
        self,
        tx: Option<mpsc::SyncSender<f32>>,
    ) -> Box<dyn FnOnce() -> io::Result<OsArchive>> {
        match self {
            SelectedImage::LocalImage(x) => Box::new(x.into_archive_fn(tx)),
            SelectedImage::RemoteImage(x) => Box::new(x.into_archive_fn(tx)),
        }
    }

    fn into_image_fn(self) -> Box<dyn FnOnce() -> io::Result<(OsImage, u64)> + Send> {
        match self {
            SelectedImage::LocalImage(x) => Box::new(x.into_image_fn()),
            SelectedImage::RemoteImage(x) => Box::new(x.into_image_fn()),
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
    chan: mpsc::SyncSender<DownloadFlashingStatus>,
    cancel_sync: bb_helper::cancel::CancellationToken,
) -> anyhow::Result<()> {
    match (img, customization, dst) {
        #[cfg(feature = "sd")]
        (BoardImage::SdFormat, _, Destination::SdCard(t)) => {
            tokio::task::spawn_blocking(move || bb_flasher::sd::FormatFlasher::new(t).flash())
                .await
                .unwrap()
        }
        #[cfg(feature = "sd")]
        (
            BoardImage::Image {
                img, bmap, flasher, ..
            },
            customization,
            Destination::LocalFile(f),
        ) if flasher == config::Flasher::SdCard => tokio::task::spawn_blocking(move || {
            bb_flasher::sd::Flasher::with_file_dest(
                img.into_image_fn(),
                bmap.map(|x| x.into_fn()),
                f,
                customization.sd_customization(),
            )
            .flash(Some(chan), Some(cancel_sync))
        })
        .await
        .unwrap(),
        #[cfg(feature = "sd")]
        (
            BoardImage::Image {
                img, bmap, flasher, ..
            },
            customization,
            Destination::SdCard(t),
        ) if flasher == config::Flasher::SdCard => tokio::task::spawn_blocking(move || {
            bb_flasher::sd::Flasher::new(
                img.into_image_fn(),
                bmap.map(|x| x.into_fn()),
                t,
                customization.sd_customization(),
            )
            .flash(Some(chan), Some(cancel_sync))
        })
        .await
        .unwrap(),
        #[cfg(feature = "sd")]
        (BoardImage::Image { img, flasher, .. }, _, Destination::SdCard(t))
            if flasher == config::Flasher::SdCardBootfs =>
        {
            let (tx, rx) = std::sync::mpsc::sync_channel(4);
            tokio::task::spawn_blocking(move || {
                while let Ok(msg) = rx.recv() {
                    let _ = chan.try_send(DownloadFlashingStatus::FlashingProgress(msg));
                }
            });
            tokio::task::spawn_blocking(move || {
                bb_flasher::sd::UpdateBootFlasher::new(
                    img.into_archive_fn(Some(tx)),
                    t,
                    Some(cancel_sync),
                )
                .flash()
            })
            .await
            .unwrap()
        }
        #[cfg(feature = "sd")]
        (BoardImage::Image { img, flasher, .. }, _, Destination::LocalFile(t))
            if flasher == config::Flasher::SdCardBootfs =>
        {
            let (tx, rx) = std::sync::mpsc::sync_channel(4);
            tokio::task::spawn_blocking(move || {
                while let Ok(msg) = rx.recv() {
                    let _ = chan.try_send(DownloadFlashingStatus::FlashingProgress(msg));
                }
            });
            tokio::task::spawn_blocking(move || {
                bb_flasher::sd::UpdateBootFlasher::with_file_dest(
                    img.into_archive_fn(Some(tx)),
                    t,
                    Some(cancel_sync),
                )
                .flash()
            })
            .await
            .unwrap()
        }
        #[cfg(feature = "bcf_cc1352p7")]
        (
            BoardImage::Image { img, .. },
            FlashingCustomization::Bcf,
            Destination::BeagleConnectFreedom(t),
        ) => tokio::task::spawn_blocking(move || {
            bb_flasher::bcf::cc1352p7::Flasher::new(img.into_image_fn(), t, true, Some(cancel_sync))
                .flash(Some(chan))
        })
        .await
        .unwrap(),
        #[cfg(feature = "bcf_msp430")]
        (BoardImage::Image { img, .. }, FlashingCustomization::Msp430, Destination::Msp430(t)) => {
            tokio::task::spawn_blocking(move || {
                bb_flasher::bcf::msp430::Flasher::new(img.into_image_fn(), t).flash(Some(chan))
            })
            .await
            .unwrap()
        }
        #[cfg(any(feature = "zepto_uart", feature = "zepto_i2c"))]
        (BoardImage::Image { img, .. }, FlashingCustomization::Zepto, Destination::Mspm0(t)) => {
            tokio::task::spawn_blocking(move || {
                bb_flasher::mspm0::Flasher::no_prep(img.into_image_fn(), t, true, Some(cancel_sync))
                    .flash(Some(chan))
            })
            .await
            .unwrap()
        }
        _ => unimplemented!(),
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) enum Destination {
    LocalFile(PathBuf),
    #[cfg(feature = "sd")]
    SdCard(bb_flasher::sd::Target),
    #[cfg(feature = "bcf_cc1352p7")]
    BeagleConnectFreedom(bb_flasher::bcf::cc1352p7::Target),
    #[cfg(feature = "bcf_msp430")]
    Msp430(bb_flasher::bcf::msp430::Target),
    #[cfg(any(feature = "zepto_uart", feature = "zepto_i2c"))]
    Mspm0(bb_flasher::mspm0::Target),
}

impl Display for Destination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Destination::LocalFile(_) => write!(f, "Save To File"),
            #[cfg(feature = "sd")]
            Destination::SdCard(target) => target.fmt(f),
            #[cfg(feature = "bcf_cc1352p7")]
            Destination::BeagleConnectFreedom(target) => target.fmt(f),
            #[cfg(feature = "bcf_msp430")]
            Destination::Msp430(target) => target.fmt(f),
            #[cfg(any(feature = "zepto_uart", feature = "zepto_i2c"))]
            Destination::Mspm0(target) => target.fmt(f),
        }
    }
}

impl bb_imager_ui::dest_selection::Destination for Destination {
    fn size(&self) -> Option<u64> {
        #[cfg(feature = "sd")]
        if let Destination::SdCard(item) = self {
            return Some(item.size());
        }

        None
    }
}

impl Destination {
    /// Download instead of flashing
    pub(crate) fn is_download_action(&self) -> bool {
        matches!(self, Self::LocalFile(_))
    }
}

pub(crate) fn destinations(
    flasher: config::Flasher,
    filter: bool,
    search: String,
) -> Box<[Destination]> {
    let filter_func =
        move |t: &Destination| search.is_empty() || t.to_string().to_lowercase().contains(&search);

    match flasher {
        #[cfg(feature = "sd")]
        config::Flasher::SdCard | config::Flasher::SdCardBootfs => {
            bb_flasher::sd::Target::destinations(filter)
                .map(Destination::SdCard)
                .filter(filter_func)
                .collect()
        }
        #[cfg(feature = "bcf_cc1352p7")]
        config::Flasher::BeagleConnectFreedom => {
            bb_flasher::bcf::cc1352p7::Target::destinations(filter)
                .map(Destination::BeagleConnectFreedom)
                .filter(filter_func)
                .collect()
        }
        #[cfg(feature = "bcf_msp430")]
        config::Flasher::Msp430Usb => bb_flasher::bcf::msp430::Target::destinations(filter)
            .map(Destination::Msp430)
            .filter(filter_func)
            .collect(),
        #[cfg(any(feature = "zepto_uart", feature = "zepto_i2c"))]
        config::Flasher::Mspm0 => bb_flasher::mspm0::Target::destinations(filter)
            .map(Destination::Mspm0)
            .filter(filter_func)
            .collect(),
        _ => unimplemented!(),
    }
}

pub(crate) fn file_filter(flasher: config::Flasher) -> &'static [&'static str] {
    match flasher {
        #[cfg(feature = "sd")]
        config::Flasher::SdCard | config::Flasher::SdCardBootfs => {
            bb_flasher::sd::Target::FILE_TYPES
        }
        #[cfg(feature = "bcf_cc1352p7")]
        config::Flasher::BeagleConnectFreedom => bb_flasher::bcf::cc1352p7::Target::FILE_TYPES,
        #[cfg(feature = "bcf_msp430")]
        config::Flasher::Msp430Usb => bb_flasher::bcf::msp430::Target::FILE_TYPES,
        #[cfg(any(feature = "zepto_uart", feature = "zepto_i2c"))]
        config::Flasher::Mspm0 => bb_flasher::mspm0::Target::FILE_TYPES,
        _ => unimplemented!(),
    }
}

pub(crate) const fn flasher_supported(flasher: config::Flasher) -> bool {
    match flasher {
        #[cfg(feature = "sd")]
        config::Flasher::SdCard | config::Flasher::SdCardBootfs => true,
        #[cfg(feature = "bcf_cc1352p7")]
        config::Flasher::BeagleConnectFreedom => true,
        #[cfg(feature = "bcf_msp430")]
        config::Flasher::Msp430Usb => true,
        #[cfg(any(feature = "zepto_uart", feature = "zepto_i2c"))]
        config::Flasher::Mspm0 => true,
        _ => false,
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub(crate) enum FlashingCustomization {
    NoneSd,
    LinuxSdSysconfig(crate::persistance::SdSysconfCustomization),
    LinuxSdCloudInit(crate::persistance::SdSysconfCustomization),
    Bcf,
    Msp430,
    Zepto,
}

impl FlashingCustomization {
    pub(crate) fn modifications(&self) -> Box<[&'static str]> {
        match self {
            FlashingCustomization::LinuxSdSysconfig(x) => {
                let mut ans = sd_modifications_common(x);
                if x.usb_enable_dhcp == Some(true) {
                    ans.push("USB DHCP enabled");
                }

                ans.into()
            }
            FlashingCustomization::LinuxSdCloudInit(x) => sd_modifications_common(x).into(),
            // The new UI drops the "Skip Verification" toggle, so Bcf/Zepto have
            // nothing to report.
            //
            // NOTE: `NoneSd` covers images with no detected init format. Eventually
            // the user should be able to pick a customization format for these, at
            // which point this arm needs to report the chosen modifications.
            FlashingCustomization::NoneSd
            | FlashingCustomization::Msp430
            | FlashingCustomization::Bcf
            | FlashingCustomization::Zepto => Box::new([]),
        }
    }

    #[cfg(feature = "sd")]
    fn sd_customization(self) -> bb_flasher::sd::FlashingSdLinuxConfig {
        match self {
            FlashingCustomization::LinuxSdSysconfig(c) => c.sysconfig(),
            FlashingCustomization::LinuxSdCloudInit(c) => c.cloudinit(),
            FlashingCustomization::NoneSd => bb_flasher::sd::FlashingSdLinuxConfig::none(),
            FlashingCustomization::Bcf
            | FlashingCustomization::Msp430
            | FlashingCustomization::Zepto => unreachable!(),
        }
    }
}

impl From<&customization::Customization> for FlashingCustomization {
    fn from(value: &customization::Customization) -> Self {
        match value {
            customization::Customization::SysConfig(x)
            | customization::Customization::SelectableSd(customization::SelectableSd::SysConfig(
                x,
            )) => FlashingCustomization::LinuxSdSysconfig(x.into()),
            customization::Customization::CloudInit(x)
            | customization::Customization::SelectableSd(customization::SelectableSd::CloudInit(
                x,
            )) => FlashingCustomization::LinuxSdCloudInit(x.into()),
            customization::Customization::SelectableSd(customization::SelectableSd::None) => {
                FlashingCustomization::NoneSd
            }
        }
    }
}

impl From<FlashingCustomization> for customization::Customization {
    fn from(value: FlashingCustomization) -> Self {
        match value {
            FlashingCustomization::LinuxSdSysconfig(x) => {
                customization::Customization::SysConfig(x.into())
            }
            FlashingCustomization::LinuxSdCloudInit(x) => {
                customization::Customization::CloudInit(x.into())
            }
            FlashingCustomization::NoneSd => {
                customization::Customization::SelectableSd(customization::SelectableSd::None)
            }
            _ => unimplemented!(),
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

pub(crate) async fn show_notification(body: String) -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    if show_notification_xdg_portal(&body).await.is_ok() {
        return Ok(());
    }

    #[cfg(feature = "notify-rust")]
    if tokio::task::spawn_blocking(move || {
        notify_rust::Notification::new()
            .appname("BeagleBoard Imager")
            .body(&body)
            .finalize()
            .show()
    })
    .await
    .unwrap()
    .is_ok()
    {
        return Ok(());
    };

    Err(anyhow::anyhow!("Failed to send notification"))
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

/// Return customization enum variant for cases where no customization is present
pub(crate) fn no_customization(
    flasher: config::Flasher,
    img: &BoardImage,
) -> Option<FlashingCustomization> {
    match flasher {
        // Formats we can actually write, plus local images, which offer the
        // format picker instead of having one detected for them.
        config::Flasher::SdCard
            if img.init_format() == config::InitFormat::Sysconf
                || img.init_format() == config::InitFormat::CloudInit
                || img.is_local() =>
        {
            None
        }
        config::Flasher::SdCard | config::Flasher::SdCardBootfs => {
            Some(FlashingCustomization::NoneSd)
        }
        config::Flasher::Msp430Usb => Some(FlashingCustomization::Msp430),
        config::Flasher::BeagleConnectFreedom => Some(FlashingCustomization::Bcf),
        config::Flasher::Mspm0 => Some(FlashingCustomization::Zepto),
        _ => unimplemented!(),
    }
}

pub(crate) fn app_title(_: &crate::BBImager) -> String {
    if cfg!(feature = "pre-release") {
        format!("{} (pre-release)", constants::APP_NAME)
    } else {
        format!("{} v{}", constants::APP_NAME, env!("CARGO_PKG_VERSION"))
    }
}

pub(crate) fn normalize_file_dest(name: &str) -> String {
    if let Some(stripped) = name.strip_suffix(".zip") {
        return stripped.to_string();
    }

    if let Some(pos) = name.rfind(".img.") {
        return name[..pos + 4].to_string();
    }

    name.to_string()
}

pub(crate) fn fetch_images(
    downloader: &bb_downloader::Downloader,
    iter: impl IntoIterator<Item = Arc<Url>>,
) -> iced::Task<BBImagerMessage> {
    let tasks = iter.into_iter().map(|icon| {
        let downloader = downloader.clone();
        // Refcount bumps; the single `Url` clone below is forced by `IntoUrl`,
        // which reqwest only implements for owned `Url`/`String`.
        let icon_msg = icon.clone();
        let url = Url::clone(&icon);
        iced::Task::perform(
            async move { downloader.download(url).await },
            move |p| match p {
                Ok(p) => BBImagerMessage::UiState(Message::ResolveImage(icon_msg, p)),
                Err(_) => {
                    tracing::warn!("Failed to fetch image {}", icon);
                    BBImagerMessage::Null
                }
            },
        )
    });

    iced::Task::batch(tasks)
}

pub(crate) fn fetch_remote_subitems(
    items: impl IntoIterator<Item = (i64, Url)>,
    downloader: bb_downloader::Downloader,
) -> iced::Task<BBImagerMessage> {
    let temp = items.into_iter().map(move |(id, url)| {
        let url_clone = url.clone();
        let dl = downloader.clone();
        iced::Task::perform(
            async move { dl.download_json_no_cache(url_clone).await },
            move |x| match x {
                Ok(json) => BBImagerMessage::ResolveRemoteSubitemItem {
                    item: json,
                    target: id,
                },
                Err(e) => {
                    tracing::error!("Failed to get remote item {}: {e}", url.as_str());
                    BBImagerMessage::Null
                }
            },
        )
    });

    iced::Task::batch(temp)
}

pub(crate) fn sd_modifications_common(
    x: &crate::persistance::SdSysconfCustomization,
) -> Vec<&'static str> {
    let mut ans = Vec::new();

    if x.user.is_some() {
        ans.push("User account configured");
    }
    if x.wifi.is_some() {
        ans.push("Wifi configured");
    }
    if x.hostname.is_some() {
        ans.push("Hostname configured");
    }
    if x.keymap.is_some() {
        ans.push("Keymap configured");
    }
    if x.timezone.is_some() {
        ans.push("Timezone configured");
    }
    if x.ssh.is_some() {
        ans.push("SSH Key configured");
    }

    ans
}

pub(crate) async fn blocking_future<F, T>(f: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f).await.unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_file_dest_strips_known_suffixes() {
        assert_eq!(normalize_file_dest("os.zip"), "os");
        assert_eq!(normalize_file_dest("os.img.xz"), "os.img");
        assert_eq!(normalize_file_dest("os.img.gz"), "os.img");
        assert_eq!(normalize_file_dest("plain.txt"), "plain.txt");
    }

    #[test]
    fn flasher_supported_matches_enabled_features() {
        // const fn whose arms are feature-gated; compare against cfg! so the
        // assertion holds under any feature set the suite is compiled with.
        assert_eq!(
            flasher_supported(config::Flasher::SdCard),
            cfg!(feature = "sd")
        );
        assert_eq!(
            flasher_supported(config::Flasher::SdCardBootfs),
            cfg!(feature = "sd")
        );
        assert_eq!(
            flasher_supported(config::Flasher::BeagleConnectFreedom),
            cfg!(feature = "bcf_cc1352p7")
        );
        assert_eq!(
            flasher_supported(config::Flasher::Msp430Usb),
            cfg!(feature = "bcf_msp430")
        );
        assert_eq!(
            flasher_supported(config::Flasher::Mspm0),
            cfg!(any(feature = "zepto_uart", feature = "zepto_i2c"))
        );
    }

    /// A remote SD image with the given init format.
    ///
    /// Remote specifically: a local image reports `InitFormat::None` and takes
    /// the `is_local` branch instead, so it cannot exercise format detection.
    fn remote_sd_image(init_format: config::InitFormat) -> BoardImage {
        let cache = tempfile::tempdir().unwrap();
        let downloader = bb_downloader::Downloader::new(cache.path()).unwrap();

        BoardImage::Image {
            flasher: config::Flasher::SdCard,
            init_format,
            img: RemoteImage::new(
                1,
                "test-image".into(),
                Box::new(url::Url::parse("https://example.com/os.img.xz").unwrap()),
                [0u8; 32],
                0,
                downloader,
            )
            .into(),
            #[cfg(feature = "sd")]
            bmap: None,
            info_text: None,
        }
    }

    /// Returning `Some` for a customizable image skips the Customize page and
    /// writes an empty config, so the user silently loses hostname, user, wifi
    /// and SSH key. Cloud-init regressed this way once already: the guard
    /// tested `Sysconf` twice instead of `Sysconf || CloudInit`.
    #[test]
    fn customizable_init_formats_reach_the_customization_page() {
        for format in [config::InitFormat::Sysconf, config::InitFormat::CloudInit] {
            let img = remote_sd_image(format);
            assert!(
                no_customization(config::Flasher::SdCard, &img).is_none(),
                "{format:?} images must be customizable"
            );
        }
    }

    /// The counterpart: formats we cannot write skip the page rather than
    /// showing one that would do nothing.
    #[test]
    fn unwritable_init_formats_skip_the_customization_page() {
        for format in [config::InitFormat::None, config::InitFormat::Armbian] {
            let img = remote_sd_image(format);
            assert!(
                matches!(
                    no_customization(config::Flasher::SdCard, &img),
                    Some(FlashingCustomization::NoneSd)
                ),
                "{format:?} images have no customization we can apply"
            );
        }
    }

    /// Local images carry no detected format, so they get the picker.
    #[test]
    fn local_images_reach_the_customization_page() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let img = BoardImage::local(file.path().to_path_buf(), config::Flasher::SdCard);

        assert!(no_customization(config::Flasher::SdCard, &img).is_none());
    }

    #[test]
    fn no_customization_covers_non_configurable_flashers() {
        let img = BoardImage::SdFormat;
        assert!(matches!(
            no_customization(config::Flasher::SdCardBootfs, &img),
            Some(FlashingCustomization::NoneSd)
        ));
        assert!(matches!(
            no_customization(config::Flasher::Msp430Usb, &img),
            Some(FlashingCustomization::Msp430)
        ));
        assert!(matches!(
            no_customization(config::Flasher::BeagleConnectFreedom, &img),
            Some(FlashingCustomization::Bcf)
        ));
    }

    #[test]
    fn board_image_format_accessors() {
        let img = BoardImage::SdFormat;
        assert_eq!(img.flasher(), config::Flasher::SdCard);
        assert_eq!(img.init_format(), config::InitFormat::None);
        assert_eq!(img.info_text(), None);
        // Nothing to write out, so the destination page offers no "Save To File".
        assert_eq!(img.file_name(), None);
        assert_eq!(img.to_string(), "Format SD Card");
    }

    #[test]
    fn board_image_local_reads_file_metadata() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), b"0123456789").unwrap();

        let img = BoardImage::local(
            file.path().to_path_buf(),
            config::Flasher::BeagleConnectFreedom,
        );
        assert_eq!(img.flasher(), config::Flasher::BeagleConnectFreedom);
        assert_eq!(img.init_format(), config::InitFormat::None);
        assert!(img.file_name().is_some_and(|n| !n.is_empty()));
    }

    #[test]
    fn destination_local_file_behaviour() {
        let dst = Destination::LocalFile(PathBuf::from("/tmp/os.img"));
        assert!(dst.is_download_action());
        assert_eq!(dst.size(), None);
        assert_eq!(dst.to_string(), "Save To File");
    }

    #[test]
    fn system_keymap_is_never_empty() {
        // Falls back to "us" when the locale cannot be resolved.
        assert!(!system_keymap().is_empty());
    }
}
