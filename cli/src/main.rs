use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
struct Opt {
    img: PathBuf,
    dst: PathBuf,
}

fn main() {
    let opt = Opt::parse();

    if !opt.img.exists() {
        eprintln!("Provided Image does not exist {:?}", opt.img);
        return;
    }

    if !opt.dst.exists() {
        eprintln!("Provided destination does not exist {:?}", opt.dst);
        return;
    }

    bb_imager::format(&opt.dst).expect("Failed to format disk");
    // flash(&opt.img, &opt.dst).expect("Failed to flash");
}
