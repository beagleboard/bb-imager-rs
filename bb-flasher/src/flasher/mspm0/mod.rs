#[cfg(feature = "mspm0_i2c")]
use std::path::PathBuf;
use std::sync::mpsc;
use std::{borrow::Cow, fmt::Display};

use bb_helper::cancel::CancellationToken;

use crate::common::{BBFlasherTarget, DownloadFlashingStatus};

/// MSPM0 UART target
#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub enum Target {
    #[cfg(feature = "mspm0_uart")]
    Uart(String),
    #[cfg(feature = "mspm0_i2c")]
    I2c(PathBuf),
}

impl Target {
    pub fn path(&self) -> String {
        match self {
            #[cfg(feature = "mspm0_uart")]
            Target::Uart(x) => x.clone(),
            #[cfg(feature = "mspm0_i2c")]
            Target::I2c(x) => x.to_string_lossy().to_string(),
        }
    }
}

impl From<String> for Target {
    #[cfg(all(feature = "mspm0_uart", feature = "mspm0_i2c"))]
    fn from(value: String) -> Self {
        if matches!(is_i2c_dev(&value), Ok(true)) {
            Self::I2c(value.into())
        } else {
            Self::Uart(value)
        }
    }

    #[cfg(all(feature = "mspm0_uart", not(feature = "mspm0_i2c")))]
    fn from(value: String) -> Self {
        Self::Uart(value)
    }

    #[cfg(all(feature = "mspm0_i2c", not(feature = "mspm0_uart")))]
    fn from(value: String) -> Self {
        Self::I2c(value.into())
    }
}

impl BBFlasherTarget for Target {
    const FILE_TYPES: &[&str] = &["bin", "hex", "txt", "xz"];

    fn destinations(_: bool) -> Vec<Self> {
        let mut dsts = Vec::new();

        #[cfg(feature = "mspm0_uart")]
        dsts.extend(bb_flasher_mspm0::uart::ports().into_iter().map(Self::Uart));

        #[cfg(all(feature = "mspm0_i2c", target_os = "linux"))]
        dsts.extend(bb_flasher_mspm0::i2c::ports().into_iter().map(Self::I2c));

        dsts
    }

    fn identifier(&self) -> Cow<'_, str> {
        match self {
            #[cfg(feature = "mspm0_uart")]
            Target::Uart(x) => Cow::Borrowed(x),
            #[cfg(feature = "mspm0_i2c")]
            Target::I2c(x) => x.to_string_lossy(),
        }
    }
}

impl Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "mspm0_uart")]
            Target::Uart(x) => x.fmt(f),
            #[cfg(feature = "mspm0_i2c")]
            Target::I2c(x) => x.display().fmt(f),
        }
    }
}

/// Flasher to flash MSPM0 Images
///
/// # Supported Image Formats
///
/// - TI-TXT
/// - iHex
/// - xz: Xz compressed files for any of the above
#[derive(Debug, Clone)]
pub struct Flasher<I, P> {
    img: I,
    port: Target,
    verify: bool,
    cancel: Option<CancellationToken>,
    prep_hook: P,
}

impl<I, P> Flasher<I, P> {
    pub fn new(
        img: I,
        port: Target,
        verify: bool,
        cancel: Option<CancellationToken>,
        prep_hook: P,
    ) -> Self {
        Self {
            img,
            port,
            verify,
            cancel,
            prep_hook,
        }
    }
}

impl<I> Flasher<I, fn() -> Result<(), bb_flasher_mspm0::Error>> {
    pub fn no_prep(img: I, port: Target, verify: bool, cancel: Option<CancellationToken>) -> Self {
        Self::new(img, port, verify, cancel, || Ok(()))
    }
}

#[cfg(target_os = "linux")]
impl<I> Flasher<I, Box<dyn FnOnce() -> bb_flasher_mspm0::Result<()> + Send>> {
    pub fn gpio_by_name(
        img: I,
        port: Target,
        verify: bool,
        cancel: Option<CancellationToken>,
        reset: String,
        bsl: String,
    ) -> Self {
        Self::new(
            img,
            port,
            verify,
            cancel,
            Box::new(bb_flasher_mspm0::bsl_gpio_cdev_by_name(reset, bsl)),
        )
    }
}

impl<I, P> Flasher<I, P>
where
    I: FnOnce() -> std::io::Result<(crate::img::OsImage, u64)> + Send + 'static,
    P: FnOnce() -> bb_flasher_mspm0::Result<()> + Send + 'static,
{
    pub fn flash(
        self,
        chan: Option<mpsc::SyncSender<DownloadFlashingStatus>>,
    ) -> anyhow::Result<()> {
        let port = self.port;
        let verify = self.verify;
        let img = self.img;
        let img = crate::common::resolve_img(img)?;

        let curry = move |chan| match port {
            #[cfg(feature = "mspm0_uart")]
            Target::Uart(x) => {
                bb_flasher_mspm0::uart::flash(&img, &x, verify, chan, self.cancel, self.prep_hook)
                    .map_err(Into::into)
            }
            #[cfg(all(feature = "mspm0_i2c", target_os = "linux"))]
            Target::I2c(x) => {
                bb_flasher_mspm0::i2c::flash(&img, &x, verify, chan, self.cancel, self.prep_hook)
                    .map_err(Into::into)
            }
            #[cfg(all(feature = "mspm0_i2c", not(target_os = "linux")))]
            Target::I2c(_) => Err(anyhow::anyhow!("Unsupported Os")),
        };

        std::thread::scope(|s| {
            if let Some(chan) = chan {
                let (tx, rx) = mpsc::sync_channel::<bb_flasher_mspm0::Status>(2);

                s.spawn(move || {
                    while let Ok(x) = rx.recv() {
                        let _ = chan.try_send(x.into());
                    }
                });

                curry(Some(tx))
            } else {
                curry(None)
            }
        })
    }
}

impl From<bb_flasher_mspm0::Status> for DownloadFlashingStatus {
    fn from(value: bb_flasher_mspm0::Status) -> Self {
        match value {
            bb_flasher_mspm0::Status::Preparing => Self::Preparing,
            bb_flasher_mspm0::Status::Flashing(x) => Self::FlashingProgress(x),
            bb_flasher_mspm0::Status::Verifying => Self::Verifying,
        }
    }
}

#[cfg(all(feature = "mspm0_i2c", target_os = "linux"))]
fn is_i2c_dev(p: impl AsRef<std::path::Path>) -> std::io::Result<bool> {
    use std::os::unix::fs::MetadataExt;

    let meta = std::fs::metadata(p)?;
    Ok(nix::sys::stat::major(meta.rdev()) == 89)
}

#[cfg(all(feature = "mspm0_i2c", not(target_os = "linux")))]
fn is_i2c_dev(_: impl AsRef<std::path::Path>) -> std::io::Result<bool> {
    Ok(false)
}
