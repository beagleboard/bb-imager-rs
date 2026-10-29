use crate::{Error, Result, helpers::Eject};

use std::io;
use std::path::{Path, PathBuf};

#[cfg(feature = "udev")]
use std::{
    collections::HashMap,
    os::fd::{FromRawFd, IntoRawFd},
    sync::{LazyLock, Mutex, PoisonError},
};

/// The one udisks2 connection, with the runtime that drives it.
///
/// polkit's agent shows one authentication dialog at a time, and a request made while one is up
/// fails rather than waiting. Every udisks2 call takes this lock for its whole duration, so
/// writing several cards at once asks for each in turn (usually once, as udisks2 keeps it).
///
/// The runtime is kept because the connection's reader task lives on it.
#[cfg(feature = "udev")]
struct Udisks {
    rt: tokio::runtime::Runtime,
    client: udisks2::Client,
}

#[cfg(feature = "udev")]
impl Udisks {
    fn connect() -> std::result::Result<Self, String> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .map_err(|e| e.to_string())?;
        let client = rt
            .block_on(udisks2::Client::new())
            .map_err(|e| e.to_string())?;
        Ok(Self { rt, client })
    }
}

/// Connected on first use. Without a system bus, connecting fails for good, so the failure is
/// kept and every call reports it.
#[cfg(feature = "udev")]
static UDISKS: LazyLock<Mutex<std::result::Result<Udisks, String>>> =
    LazyLock::new(|| Mutex::new(Udisks::connect()));

/// Run `f` with the shared client, after any other udisks2 call has finished.
#[cfg(feature = "udev")]
fn with_udisks<T>(f: impl AsyncFnOnce(&udisks2::Client) -> io::Result<T>) -> io::Result<T> {
    let guard = UDISKS.lock().unwrap_or_else(PoisonError::into_inner);
    let Udisks { rt, client } = guard.as_ref().map_err(|e| io::Error::other(e.clone()))?;
    rt.block_on(f(client))
}

/// The udisks2 block device for `dst`.
#[cfg(feature = "udev")]
async fn block_for(
    client: &udisks2::Client,
    dst: &Path,
) -> io::Result<udisks2::block::BlockProxy<'static>> {
    let devs = client
        .manager()
        .resolve_device(
            HashMap::from([("path", dst.to_str().unwrap().into())]),
            HashMap::new(),
        )
        .await
        .map_err(io::Error::other)?;

    let block = devs
        .first()
        .ok_or(io::Error::new(
            io::ErrorKind::NotFound,
            "Block device not found",
        ))?
        .to_owned();

    client
        .object(block)
        .expect("Unexpected error")
        .block()
        .await
        .map_err(io::Error::other)
}

#[cfg(feature = "udev")]
pub(crate) fn format(dst: &Path) -> Result<()> {
    with_udisks(async |client| {
        block_for(client, dst)
            .await?
            .format(
                "vfat",
                HashMap::from([("update-partition-type", true.into())]),
            )
            .await
            .map_err(io::Error::other)
    })
    .map_err(|source| Error::FailedToFormat { source })
}

#[cfg(feature = "udev")]
pub(crate) fn open(dst: &Path) -> Result<LinuxDrive> {
    with_udisks(async |client| {
        let fd = block_for(client, dst)
            .await?
            .open_device("rw", HashMap::from([("flags", libc::O_DIRECT.into())]))
            .await
            .map_err(io::Error::other)?;
        let file =
            unsafe { std::fs::File::from_raw_fd(std::os::fd::OwnedFd::from(fd).into_raw_fd()) };

        Ok(LinuxDrive {
            file,
            drive: dst.to_path_buf(),
        })
    })
    .map_err(|e| Error::FailedToOpenDestination { source: e.into() })
}

#[cfg(not(feature = "udev"))]
pub(crate) fn open(dst: &Path) -> Result<LinuxDrive> {
    use std::os::unix::fs::OpenOptionsExt;

    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(false)
        .custom_flags(libc::O_DIRECT)
        .open(dst)?;

    Ok(LinuxDrive {
        file,
        drive: dst.to_path_buf(),
    })
}

#[cfg(not(feature = "udev"))]
pub(crate) fn format(dst: &Path) -> Result<()> {
    let sd = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(false)
        .open(dst)?;

    fatfs::format_volume(sd, fatfs::FormatVolumeOptions::default())
        .map_err(|source| Error::FailedToFormat { source })
}

#[derive(Debug)]
pub(crate) struct LinuxDrive {
    file: std::fs::File,
    drive: PathBuf,
}

#[cfg(feature = "udev")]
impl Eject for LinuxDrive {
    fn eject(self) -> io::Result<()> {
        let _ = self.file.sync_all();
        let dst = self.drive.clone();

        std::mem::drop(self);

        with_udisks(async |client| {
            let drive = block_for(client, &dst)
                .await?
                .drive()
                .await
                .map_err(io::Error::other)?;

            client
                .object(drive)
                .expect("Unexpected error")
                .drive()
                .await
                .map_err(io::Error::other)?
                .eject(HashMap::new())
                .await
                .map_err(io::Error::other)
        })
    }
}

#[cfg(not(feature = "udev"))]
impl Eject for LinuxDrive {
    fn eject(self) -> std::io::Result<()> {
        let _ = self.file.sync_all();
        let drive = self.drive.clone();
        std::mem::drop(self);

        let output = std::process::Command::new("eject").arg(drive).output()?;
        if output.status.success() {
            Ok(())
        } else {
            Err(std::io::Error::other(
                String::from_utf8(output.stderr).unwrap(),
            ))
        }
    }
}

impl io::Read for LinuxDrive {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }
}

impl io::Seek for LinuxDrive {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.file.seek(pos)
    }
}

impl io::Write for LinuxDrive {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
