# SD Card Flasher

Library to flash SD cards with OS images. Powers sd card flashing in [BeagleBoard Imager](https://openbeagle.org/ayush1325/bb-imager-rs).

Also allows optional extra Customization for BeagleBoard images.

## Platform Support

- Linux
- Windows
- MacOS

## Features

- `udev`: Dynamic permissions on Linux. Mostly useful for GUI and flatpaks
- `macos_authopen`: Dynamic permissions on MacOS.

## Usage

```rust
use std::path::PathBuf;
use std::fs::File;

#[tokio::main]
async fn main() {
    let dst = PathBuf::from("/tmp/dummy").into();
    let img = bb_helper::resolvable::LocalFile::new(PathBuf::from("/tmp/image").into());
    let (tx, mut rx) = tokio::sync::mpsc::channel(20);

    let flash_thread = tokio::spawn(async move { bb_flasher_sd::flash(img, None::<bb_helper::resolvable::LocalStringFile>, dst, Some(tx), None, None).await });

    while let Some(m) = rx.recv().await {
        println!("{:?}", m);
    }

    flash_thread.await.unwrap().unwrap()
}
```
