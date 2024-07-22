use std::{io, path::Path};

pub fn flash(img: &Path, dev: &Path) -> io::Result<()> {
    std::fs::copy(img, dev).map(|_| ())
}

pub fn format(dev: &Path) -> io::Result<()> {
    let disk = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(dev)?;
    fatfs::format_volume(disk, fatfs::FormatVolumeOptions::new())
}
