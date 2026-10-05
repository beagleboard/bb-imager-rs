# bb-flasher-mspm0

A library to flash the MSPM0 co-processor using its ROM bootloader (BSL). Firmware can be provided as a raw binary or as Intel HEX / Motorola S-Record / TI-TXT text.

# Features

- `uart` (default) – `bb_flasher_mspm0::uart`: flash over a serial port. Cross-platform.
- `i2c` – `bb_flasher_mspm0::i2c`: flash over an I2C bus. Linux only.

On Linux, `bsl_gpio_cdev_by_name` returns a prep hook that puts MSPM0 into BSL mode by toggling the named BSL and reset GPIO lines.

# Sample Usage

```toml
[dependencies]
bb-flasher-mspm0 = "0.1"
```

```rust,no_run
use std::sync::mpsc;

use bb_flasher_mspm0::{Status, uart};

let firmware = std::fs::read("firmware.hex").unwrap();
let port = uart::ports().next().expect("No serial port found");

let (tx, rx) = mpsc::sync_channel(8);
std::thread::spawn(move || {
    for status in rx {
        match status {
            Status::Preparing => println!("Preparing"),
            Status::Flashing(p) => println!("Flashing: {:.0}%", p * 100.0),
            Status::Verifying => println!("Verifying"),
        }
    }
});

// Assumes MSPM0 is already in BSL mode, so the prep hook does nothing.
uart::flash(&firmware, &port, true, Some(tx), None, || Ok(())).unwrap();
```

# Helpful Links

- [MSPM0 Bootloader User's Guide](https://www.ti.com/lit/ug/slau887/slau887.pdf)
