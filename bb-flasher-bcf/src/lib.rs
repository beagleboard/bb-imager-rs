//! Library to provide flashing capabilities for [BeagleConnect Freedom]. This includes both
//! [CC1352P7] and [MSP430F5503], which serves as the USB to UART bridge.
//!
//! [BeagleConnect Freedom]: https://www.beagleboard.org/boards/beagleconnect-freedom
//! [CC1352P7]: https://www.ti.com/product/CC1352P7
//! [MSP430F5503]: https://www.ti.com/product/MSP430F5503

#[cfg(feature = "cc1352p7")]
pub mod cc1352p7;
pub(crate) mod helpers;
#[cfg(feature = "msp430")]
pub mod msp430;

/// Flashing status
pub enum Status {
    Preparing,
    Flashing(f32),
    Verifying,
}
