//! Abstractions to parse and generate distros.json file.

use std::collections::HashSet;

use semver::Version;
use serde::{Deserialize, Serialize};
use serde_with::{VecSkipError, serde_as};
use url::Url;

/// [BeagleBoard.org] distros.json abstraction.
///
/// # Merging Behaviour
///
/// Configs can be merged together using [Extend::extend]. This allows for having some portion of
/// config locally while downloading parts from network.
///
/// If [Imager::latest_version] is present, it will overwrite the current field.
///
/// For [Imager::devices], any new board will be appended to the end of the board list. Existing
/// boards are not added again. Duplicate boards are checked by [Device::name] field. The [Device]
/// fields are overwritten.
///
/// For [Config::os_list], all non-duplicate [OsListItem] are appended to the end of the list.
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
    /// Latest BeagleBoard Imaging Utility version
    pub latest_version: Option<Version>,
    /// A list of remote config files
    #[serde(default)]
    pub remote_configs: HashSet<Url>,
    #[serde_as(as = "VecSkipError<_>")]
    #[serde(default)]
    /// List of BeagleBoard.org boards
    pub devices: Vec<Device>,
}

/// Structure describing [BeagleBoard.org] board
///
/// [BeagleBoard.org]: https://www.beagleboard.org/
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
}

/// Os List can contain multiple types of items depending on the situation.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
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
}

/// Types of flashers Os Image(s) support
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[non_exhaustive]
pub enum Flasher {
    #[default]
    /// Image needs to be written to SD Card
    SdCard,
    /// BeagleConnect Freedom CC1352P7 Firmware
    BeagleConnectFreedom,
    /// BeagleConnect Freedom Msp430 Firmware
    Msp430Usb,
    /// PocketBeagle2 Mspm0 firmware
    Pb2Mspm0,
}

impl Extend<Self> for Config {
    fn extend<T: IntoIterator<Item = Self>>(&mut self, iter: T) {
        for config in iter.into_iter() {
            // Overwrite version if present
            if let Some(v) = config.imager.latest_version {
                self.imager.latest_version = Some(v);
            }

            self.imager
                .remote_configs
                .extend(config.imager.remote_configs);

            for c_dev in config.imager.devices {
                // If the board already exists, overwrite fields.
                // Else, add new board
                if let Some(my_dev) = self
                    .imager
                    .devices
                    .iter_mut()
                    .find(|x| x.name == c_dev.name)
                {
                    my_dev.tags.extend(c_dev.tags.clone());
                    my_dev.flasher = c_dev.flasher;

                    if let Some(doc) = &c_dev.documentation {
                        my_dev.documentation = Some(doc.clone());
                    }

                    if let Some(icon) = &c_dev.icon {
                        my_dev.icon = Some(icon.clone());
                    }
                } else {
                    self.imager.devices.push(c_dev);
                }
            }

            // Only add non_duplicate os_list items
            self.os_list.reserve(config.os_list.len());
            for item in config.os_list {
                if !self.os_list.contains(&item) {
                    self.os_list.push(item);
                }
            }
        }
    }
}

impl OsListItem {
    pub fn icon(&self) -> url::Url {
        match self {
            OsListItem::Image(img) => img.icon.clone(),
            OsListItem::SubList(img) => img.icon.clone(),
            OsListItem::RemoteSubList(img) => img.icon.clone(),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            OsListItem::Image(img) => &img.name,
            OsListItem::SubList(img) => &img.name,
            OsListItem::RemoteSubList(img) => &img.name,
        }
    }

    /// Check if the [OsListItem] (or any of it's children) has an image for a board
    pub fn has_board_image(&self, tags: &HashSet<String>) -> bool {
        match self {
            OsListItem::Image(item) => !tags.is_disjoint(&item.devices),
            OsListItem::SubList(item) => item.subitems.iter().any(|x| x.has_board_image(tags)),
            OsListItem::RemoteSubList(item) => !tags.is_disjoint(&item.devices),
        }
    }
}

impl OsRemoteSubList {
    /// Construct [OsSubList] once subitems have been downloaded.
    pub fn resolve(self, subitems: Vec<OsListItem>) -> OsSubList {
        OsSubList {
            name: self.name,
            description: self.description,
            icon: self.icon,
            flasher: self.flasher,
            subitems,
        }
    }
}
