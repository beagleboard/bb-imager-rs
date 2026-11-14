#![cfg(feature = "fmt")]

use bb_helper::fmt::pretty_bytes;

#[test]
fn pretty_bytes_scales_units() {
    assert_eq!(&*pretty_bytes(0), "0 B");
    assert_eq!(&*pretty_bytes(512), "512 B");
    assert_eq!(&*pretty_bytes(1024), "1.00 KiB");
    assert_eq!(&*pretty_bytes(1536), "1.50 KiB");
    assert_eq!(&*pretty_bytes(1024 * 1024), "1.00 MiB");
    assert_eq!(&*pretty_bytes(1024 * 1024 * 1024), "1.00 GiB");
}
