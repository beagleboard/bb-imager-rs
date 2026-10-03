# BB Helper

Small helper utilities shared across the [BeagleBoard.org imaging tools](https://github.com/beagleboard/bb-imager-rs).

All functionality is behind feature flags, so only what is enabled gets compiled.

# Features

- `cancel` – `bb_helper::cancel`: a lightweight, thread-safe `CancellationToken` that is cancelled when its `DropGuard` is dropped.
- `reader_progress` – `bb_helper::reader_progress`: `ReaderWithProgress`, a `Read`/`Seek` wrapper that reports progress (`0.0..=1.0`) over a channel.
- `file_stream` – `bb_helper::file_stream`: a temporary-file-backed stream with an async writer half and a blocking reader half, for data too large to keep in memory.

# Sample Usage

```toml
[dependencies]
bb-helper = { version = "0.1", features = ["cancel", "reader_progress"] }
```

```rust
use std::io::Read;
use std::sync::mpsc;

use bb_helper::cancel::CancellationToken;
use bb_helper::reader_progress::ReaderWithProgress;

let token = CancellationToken::default();
let guard = token.drop_guard();

let data = [0u8; 64];
let (tx, rx) = mpsc::sync_channel(8);
let mut reader = ReaderWithProgress::new(&data[..], data.len() as u64, Some(tx));

let mut buf = [0u8; 32];
while !token.is_cancelled() && reader.read(&mut buf).unwrap() != 0 {}
assert_eq!(rx.try_iter().last(), Some(1.0));

// Dropping the guard cancels the token (and all its clones).
drop(guard);
assert!(token.is_cancelled());
```
