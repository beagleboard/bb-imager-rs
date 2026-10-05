# bb-flasher-pb2-mspm0

A library to flash MSPM0 co-processor in [PocketBeagle 2](https://www.beagleboard.org/boards/pocketbeagle-2). It uses the kernel driver which supports [Linux Firmware Upload API](https://docs.kernel.org/driver-api/firmware/fw_upload.html).

# Sample Usage

```toml
[dependencies]
bb-flasher-pb2-mspm0 = "0.1"
```

```rust,no_run
use std::sync::mpsc;

use bb_flasher_pb2_mspm0::{Status, check, flash};

let firmware = std::fs::read("firmware.bin").unwrap();

// Ensure the firmware upload sysfs entries are present.
check().unwrap();

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

// Flash the firmware while preserving the EEPROM contents.
flash(&firmware, tx, true).unwrap();
```

# Helpful Links

- [PocketBeagle 2](https://www.beagleboard.org/boards/pocketbeagle-2)
- [Linux Firmware Upload API](https://docs.kernel.org/driver-api/firmware/fw_upload.html)
