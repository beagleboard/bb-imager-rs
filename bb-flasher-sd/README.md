# SD Card Flasher

Library to flash SD cards with OS images. Powers sd card flashing in [BeagleBoard Imager](https://openbeagle.org/ayush1325/bb-imager-rs).

Also allows optional extra Customization for BeagleBoard images. Currently only supports sysconf based post-install configuration.

## Platform Support

- Linux
- Windows
- MacOS

## Features

- `udev`: Dynamic permissions on Linux. Mostly useful for GUI and flatpaks
- `macos_authopen`: Dynamic permissions on MacOS.

## Usage

```rust
use std::path::Path;
use std::fs::File;

fn main() {
    let dst = Path::new("/tmp/dummy");
    let img = || {
        Ok((File::open("/tmp/image")?, 1024))
    };
    let (tx, rx) = futures::channel::mpsc::channel(20);

    let flash_thread = std::thread::spawn(move || {
        bb_flasher_sd::flash(
            img,
            dst,
            true,
            Some(tx),
            None,
            None
        )
    });

    let msgs = futures::executor::block_on_stream(rx);
    for m in msgs {
        println!("{:?}", m);
    }

    flash_thread.join().unwrap().unwrap()
}
```
