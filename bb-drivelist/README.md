# bb-drivelist

A Rust implementation of Balena's drivelist for enumerating storage drives across platforms.

This crate provides a simple and unified API to list all available drives on:

- Linux
- Windows
- macOS

## Usage

```rust
fn main() {
    println!("{:#?}", bb_drivelist::drive_list());
}
```
