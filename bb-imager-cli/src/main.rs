use clap::Parser;
use std::{
    io,
    path::{Path, PathBuf},
};
use tracing::{error, info};

#[derive(Parser)]
struct Opt {
    img: PathBuf,
    dst: PathBuf,
}

fn main() {
    let opt = Opt::parse();

    tracing_subscriber::fmt().init();

    if !opt.img.exists() {
        error!("Provided Image does not exist {:?}", opt.img);
        return;
    }

    if !opt.dst.exists() {
        error!("Provided destination does not exist {:?}", opt.dst);
        return;
    }

    format(&opt.dst).expect("Failed to format disk");
    // flash(&opt.img, &opt.dst).expect("Failed to flash");
}

fn flash(img: &Path, dev: &Path) -> io::Result<()> {
    info!("Writing image {:?} to {:?}", img, dev);
    std::fs::copy(img, dev).map(|_| ())
}

fn format(dev: &Path) -> io::Result<()> {
    info!("Formatting device to fat32");
    let disk = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(dev)?;
    fatfs::format_volume(disk, fatfs::FormatVolumeOptions::new())
}
