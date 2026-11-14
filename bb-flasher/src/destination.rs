//! A single type over every flasher target enabled by features.

use std::borrow::Cow;
use std::fmt::Display;
use std::path::Path;

#[allow(unused_imports)]
use crate::BBFlasherTarget;

/// Any flasher target enabled by features, or a local file to write the image to.
///
/// Variants come and go with features, so matches outside this crate need a wildcard arm.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
#[non_exhaustive]
pub enum Destination {
    /// Write the image to a file instead of flashing a device.
    LocalFile(Box<Path>),
    /// An SD card.
    #[cfg(feature = "sd")]
    SdCard(crate::sd::Target),
    /// BeagleConnect Freedom main processor (CC1352P7).
    #[cfg(feature = "bcf")]
    BeagleConnectFreedom(crate::bcf::cc1352p7::Target),
    /// BeagleConnect Freedom MSP430 USB to UART bridge.
    #[cfg(feature = "bcf_msp430")]
    Msp430(crate::bcf::msp430::Target),
    /// MSPM0 over UART or I2C.
    #[cfg(any(feature = "mspm0_uart", feature = "mspm0_i2c"))]
    Mspm0(crate::mspm0::Target),
    /// PocketBeagle 2 MSPM0.
    #[cfg(feature = "pb2_mspm0")]
    Pb2Mspm0(crate::pb2::mspm0::Target),
    /// A USB DFU device.
    #[cfg(feature = "dfu")]
    Dfu(crate::dfu::Target),
}

impl Display for Destination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LocalFile(_) => write!(f, "Save To File"),
            #[cfg(feature = "sd")]
            Self::SdCard(t) => t.fmt(f),
            #[cfg(feature = "bcf")]
            Self::BeagleConnectFreedom(t) => t.fmt(f),
            #[cfg(feature = "bcf_msp430")]
            Self::Msp430(t) => t.fmt(f),
            #[cfg(any(feature = "mspm0_uart", feature = "mspm0_i2c"))]
            Self::Mspm0(t) => t.fmt(f),
            #[cfg(feature = "pb2_mspm0")]
            Self::Pb2Mspm0(t) => t.fmt(f),
            #[cfg(feature = "dfu")]
            Self::Dfu(t) => t.fmt(f),
        }
    }
}

impl Destination {
    /// Size in bytes, for destinations that have one (SD cards).
    pub const fn size(&self) -> Option<u64> {
        #[cfg(feature = "sd")]
        if let Self::SdCard(item) = self {
            return Some(item.size());
        }

        None
    }

    /// Display name, with the size appended where there is one, e.g. `"SD Card (29.72 GiB)"`.
    pub fn label(&self) -> Box<str> {
        match self.size() {
            Some(x) => format!("{} ({})", self, bb_helper::fmt::pretty_bytes(x)).into(),
            None => self.to_string().into(),
        }
    }

    /// Stable identity of this destination, for matching a selection back to a device after
    /// the list has been re-enumerated.
    pub fn identifier(&self) -> Cow<'_, str> {
        match self {
            Self::LocalFile(p) => p.to_string_lossy(),
            #[cfg(feature = "sd")]
            Self::SdCard(t) => t.identifier(),
            #[cfg(feature = "bcf")]
            Self::BeagleConnectFreedom(t) => t.identifier(),
            #[cfg(feature = "bcf_msp430")]
            Self::Msp430(t) => t.identifier(),
            #[cfg(any(feature = "mspm0_uart", feature = "mspm0_i2c"))]
            Self::Mspm0(t) => t.identifier(),
            #[cfg(feature = "pb2_mspm0")]
            Self::Pb2Mspm0(t) => t.identifier(),
            #[cfg(feature = "dfu")]
            Self::Dfu(t) => t.identifier(),
        }
    }
}
