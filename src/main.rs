use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
struct Opt {
    img: PathBuf,
    dst: PathBuf,
}

fn main() {
    let opt = Opt::parse();
    std::fs::copy(opt.img, opt.dst).unwrap();
}
