#[test]
#[cfg(feature = "uart")]
fn serial_ports() {
    bb_flasher_mspm0::uart::ports();
}

#[test]
#[cfg(all(feature = "i2c", target_os = "linux"))]
fn i2c_ports() {
    bb_flasher_mspm0::i2c::ports();
}
