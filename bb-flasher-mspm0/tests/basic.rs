#[test]
#[cfg(feature = "uart")]
fn serial_ports() {
    let _ = bb_flasher_mspm0::uart::ports().collect::<Vec<_>>();
}

#[test]
#[cfg(all(feature = "i2c", target_os = "linux"))]
fn i2c_ports() {
    let _ = bb_flasher_mspm0::i2c::ports().collect::<Vec<_>>();
}
