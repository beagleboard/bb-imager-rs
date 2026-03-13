# bb-helper

Common helper utilities used across the BeagleBoard imaging tools.

This crate is a small shared library that provides:

- A file-backed stream splitter (`file_stream`) that exposes an async writer and a synchronous reader.
- A `Resolvable` trait (`resolvable`) for representing image sources that can be resolved to local files.

## Features

- `file_stream` – enables `bb_helper::file_stream` and related types.
- `resolvable` – enables `bb_helper::resolvable` and related types.

## Usage

Add `bb-helper` as a dependency and enable the feature(s) you need:

```toml
[dependencies]
bb-helper = { path = "../bb-helper", features = ["file_stream", "resolvable"] }
```

Then use the available types in your crate:

```rust
#[cfg(feature = "file_stream")]
use bb_helper::file_stream;

#[cfg(feature = "resolvable")]
use bb_helper::resolvable::LocalFile;
```
