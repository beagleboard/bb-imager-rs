use clap::Parser;
use std::path::PathBuf;
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

    info!("Writing image {:?} to {:?}", opt.img, opt.dst);
    std::fs::copy(opt.img, opt.dst).unwrap();
}
