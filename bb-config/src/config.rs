//! Abstractions to parse and generate distros.json file.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_with::{Map, VecSkipError, serde_as};
use url::Url;

/// [BeagleBoard.org] distros.json abstraction.
///
/// [BeagleBoard.org]: https://www.beagleboard.org/
#[serde_as]
#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct Config {
    #[serde(default)]
    pub imager: Imager,
    #[serde_as(as = "VecSkipError<_>")]
    /// List of OS images for the boards
    pub os_list: Vec<OsListItem>,
}

/// Contains information regarding BeagleBoard Images version and a list of [BeagleBoard.org]
/// boards along with information regarding each board.
///
/// [BeagleBoard.org]: https://www.beagleboard.org/
#[serde_as]
#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct Imager {
    /// A list of remote config files
    #[serde(default)]
    pub remote_configs: Vec<Url>,
    #[serde_as(as = "VecSkipError<_>")]
    #[serde(default)]
    /// List of BeagleBoard.org boards
    pub devices: Vec<Device>,
}

/// Structure describing [BeagleBoard.org] board
///
/// [BeagleBoard.org]: https://www.beagleboard.org/
#[serde_as]
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Device {
    /// Board Name
    pub name: String,
    /// Board tags are used to match OS images with boards
    pub tags: HashSet<String>,
    /// Board image URL
    pub icon: Option<Url>,
    /// Board description
    pub description: String,
    /// The default [`Flasher`] for the board. This will be used when flasher type is not present
    /// in the OS image.
    pub flasher: Flasher,
    /// Link to board documentation
    pub documentation: Option<Url>,
    /// Special Instructions for flashing board.
    pub instructions: Option<String>,
    #[serde(default)]
    #[serde_as(as = "Map<_, _>")]
    /// Board Specification. With order preserved
    pub specification: Vec<(String, String)>,
    /// OSHW details for the device.
    pub oshw: Option<String>,
}

/// Types of customization Initialization formats
#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
#[serde(rename_all = "lowercase")]
pub enum InitFormat {
    #[default]
    None,
    /// Sysconfig based customization
    Sysconf,
    /// Armbian base customization
    Armbian,
    /// Cloud Init based customization
    CloudInit,
}

impl rusqlite::ToSql for InitFormat {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let val: u8 = match self {
            InitFormat::None => 1,
            InitFormat::Sysconf => 2,
            InitFormat::Armbian => 3,
            InitFormat::CloudInit => 4,
        };
        Ok(rusqlite::types::ToSqlOutput::from(val))
    }
}

impl rusqlite::types::FromSql for InitFormat {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        value.as_i64().and_then(|val| match val {
            1 => Ok(InitFormat::None),
            2 => Ok(InitFormat::Sysconf),
            3 => Ok(InitFormat::Armbian),
            4 => Ok(InitFormat::CloudInit),
            _ => Err(rusqlite::types::FromSqlError::Other(
                format!("Invalid InitFormat integer variant: {}", val).into(),
            )),
        })
    }
}

impl std::fmt::Display for InitFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitFormat::None => f.write_str("none"),
            InitFormat::Sysconf => f.write_str("sysconfig"),
            InitFormat::Armbian => f.write_str("armbian"),
            InitFormat::CloudInit => f.write_str("cloudinit"),
        }
    }
}

/// Os List can contain multiple types of items depending on the situation.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum OsListItem {
    /// Single Os Image
    Image(OsImage),
    /// SubList which itself can contain a list of [`OsListItem`].
    ///
    /// This is used to define Testing and other images which do not need to be present at the top
    /// level.
    SubList(OsSubList),
    /// SubList stored in a remote location.
    ///
    /// This is used to define images managed/hosted outside of the normal [BeagleBoard.org] image
    /// infrastructure, such as from CI, etc.
    ///
    /// [BeagleBoard.org]: https://www.beagleboard.org/
    RemoteSubList(OsRemoteSubList),
}

/// [`OsListItem`] which itself can contain a list of [`OsListItem`].
#[serde_as]
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct OsSubList {
    /// Sublist name
    pub name: String,
    /// Sublist description
    pub description: String,
    /// Sublist icon URL
    pub icon: Url,
    /// Flasher type for all top level Os Images in the sublist
    #[serde(default)]
    pub flasher: Flasher,
    /// List of items
    #[serde_as(as = "VecSkipError<_>")]
    pub subitems: Vec<OsListItem>,
}

/// Sublists stored in a remote location
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct OsRemoteSubList {
    /// Remote Sublist name
    pub name: String,
    /// Remote Sublist description
    pub description: String,
    /// Remote Sublist icon URL
    pub icon: Url,
    /// Flasher type for all top level Os Images in the sublist
    #[serde(default)]
    pub flasher: Flasher,
    /// Union of devices the OsImages in the SubList can be used with
    pub devices: HashSet<String>,
    /// Url to the Remote list
    pub subitems_url: Url,
}

/// A singular Os Image for board(s)
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct OsImage {
    /// Os Image name
    pub name: String,
    /// Os Image description
    pub description: String,
    /// Os Image icon
    pub icon: Url,
    /// Os Image download URL
    pub url: Url,
    /// Os Image size before download
    pub image_download_size: Option<u64>,
    /// Os Image sha256 (before extraction)
    #[serde(with = "const_hex")]
    pub image_download_sha256: [u8; 32],
    /// Os Image size after extraction
    pub extract_size: u64,
    /// Os Image release date
    pub release_date: chrono::NaiveDate,
    /// Devices the Os Image can be used with
    pub devices: HashSet<String>,
    /// Os Image tags
    #[serde(default)]
    pub tags: HashSet<String>,
    /// Initialization Format. Currently only used by SD Card Images
    #[serde(default)]
    pub init_format: InitFormat,
    /// Bmap file for the image
    pub bmap: Option<Url>,
    /// Special Instructions for flashing board.
    pub info_text: Option<String>,
    /// URL to support page for image. This is where issues should be reported.
    pub support: Option<Url>,
}

/// Types of flashers Os Image(s) support
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Flasher {
    #[default]
    /// Image needs to be written to SD Card
    SdCard,
    /// Archive for updated bootfs
    SdCardBootfs,
    /// BeagleConnect Freedom CC1352P7 Firmware
    BeagleConnectFreedom,
    /// BeagleConnect Freedom Msp430 Firmware
    Msp430Usb,
    /// PocketBeagle2 Mspm0 firmware
    Pb2Mspm0,
    /// MSPM0 flasher
    Mspm0,
}

impl rusqlite::ToSql for Flasher {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let val: u8 = match self {
            Flasher::SdCard => 1,
            Flasher::SdCardBootfs => 2,
            Flasher::BeagleConnectFreedom => 3,
            Flasher::Msp430Usb => 4,
            Flasher::Pb2Mspm0 => 5,
            Flasher::Mspm0 => 6,
        };

        Ok(rusqlite::types::ToSqlOutput::from(val))
    }
}

impl rusqlite::types::FromSql for Flasher {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        value.as_i64().and_then(|val| match val {
            1 => Ok(Flasher::SdCard),
            2 => Ok(Flasher::SdCardBootfs),
            3 => Ok(Flasher::BeagleConnectFreedom),
            4 => Ok(Flasher::Msp430Usb),
            5 => Ok(Flasher::Pb2Mspm0),
            6 => Ok(Flasher::Mspm0),
            _ => Err(rusqlite::types::FromSqlError::Other(
                format!("Invalid Flasher discriminant: {}", val).into(),
            )),
        })
    }
}
