# bb-flasher-pb2-mspm0

A library to flash MSPM0 co-processor in [PocketBeagle 2](https://www.beagleboard.org/boards/pocketbeagle-2). It uses the kernel driver which supports [Linux Firmware Upload API](https://docs.kernel.org/driver-api/firmware/fw_upload.html).

# Sample Usage

```toml
[dependencies]
bb-flasher-pb2-mspm0 = "0.2"
```

```rust,no_run
use std::sync::mpsc;

use bb_flasher_pb2_mspm0::{Status, flash};

let firmware = std::fs::read("firmware.bin").unwrap();

let (tx, rx) = mpsc::sync_channel(8);
std::thread::spawn(move || {
    for status in rx {
        match status {
            Status::Preparing => println!("Preparing"),
            Status::Writing(p) => println!("Writing: {:.0}%", p * 100.0),
            Status::Verifying => println!("Verifying"),
            Status::Customizing => {}
        }
    }
});

// Flash the firmware while preserving the EEPROM contents.
flash(&firmware, Some(tx), true).unwrap();
```

# Helpful Links

- [PocketBeagle 2](https://www.beagleboard.org/boards/pocketbeagle-2)
- [Linux Firmware Upload API](https://docs.kernel.org/driver-api/firmware/fw_upload.html)
