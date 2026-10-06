# bb-flasher-dfu

A library to flash BeagleBoard devices over [USB DFU](https://www.usb.org/sites/default/files/DFU_1.1.pdf). Images are downloaded in order, each to the DFU interface matching its name (e.g. `tiboot3.bin`, `tispl.bin`, `u-boot.img`).

# Sample Usage

```toml
[dependencies]
bb-flasher-dfu = "0.1"
```

```rust,no_run
use std::fs::File;
use std::sync::mpsc;

let dev = bb_flasher_dfu::devices(true)
    .into_iter()
    .next()
    .expect("No DFU device found");

let imgs: Vec<_> = ["tiboot3.bin", "tispl.bin", "u-boot.img"]
    .into_iter()
    .map(|name| {
        let opener = move || {
            let f = File::open(name)?;
            let size = f.metadata()?.len();
            Ok((f, size))
        };
        (name.to_string(), opener)
    })
    .collect();

let (tx, rx) = mpsc::sync_channel(8);
std::thread::spawn(move || {
    for p in rx {
        println!("Progress: {:.0}%", p * 100.0);
    }
});

bb_flasher_dfu::flash(
    imgs,
    dev.vendor_id,
    dev.product_id,
    dev.bus_num,
    dev.port_num,
    Some(tx),
    None,
)
.unwrap();
```

# Helpful Links

- [USB Device Firmware Upgrade 1.1](https://www.usb.org/sites/default/files/DFU_1.1.pdf)
