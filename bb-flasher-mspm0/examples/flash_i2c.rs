use std::{env, fs, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let mut args = env::args().skip(1);
    let image = args.next().ok_or("usage: flash_i2c <image.bin> <i2c-dev>")?;
    let port = args.next().ok_or("usage: flash_i2c <image.bin> <i2c-dev>")?;
    if args.next().is_some() {
        return Err("usage: flash_i2c <image.bin> <i2c-dev>".into());
    }

    let image = fs::read(image)?;
    bb_flasher_mspm0::i2c::flash(&image, Path::new(&port), true, None, None)?;
    Ok(())
}
