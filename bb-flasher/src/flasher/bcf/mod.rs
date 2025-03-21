//! Provide flashing capabilities for [BeagleConnect Freedom]. This includes both [CC1352P7] and
//! [MSP430F5503].
//!
//! [BeagleConnect Freedom]: https://www.beagleboard.org/boards/beagleconnect-freedom
//! [CC1352P7]: https://www.ti.com/product/CC1352P7
//! [MSP430F5503]: https://www.ti.com/product/MSP430F5503

#[cfg(feature = "bcf")]
pub mod cc1352p7;
#[cfg(feature = "bcf_msp430")]
pub mod msp430;

impl From<bb_flasher_bcf::Status> for crate::DownloadFlashingStatus {
    fn from(value: bb_flasher_bcf::Status) -> Self {
        match value {
            bb_flasher_bcf::Status::Preparing => Self::Preparing,
            bb_flasher_bcf::Status::Flashing(x) => Self::FlashingProgress(x),
            bb_flasher_bcf::Status::Verifying => Self::Verifying,
        }
    }
}
