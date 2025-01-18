//! Configuration for bb-imager to use.

use std::collections::HashSet;

use semver::Version;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, VecSkipError};
use url::Url;

use crate::common::Flasher;

#[serde_as]
#[derive(Deserialize, Debug, Clone, Default)]
pub struct Config {
    pub imager: Imager,
    #[serde_as(as = "VecSkipError<_>")]
    pub os_list: Vec<OsListItem>,
}

#[serde_as]
#[derive(Deserialize, Debug, Clone, Default)]
pub struct Imager {
    pub latest_version: Option<Version>,
    #[serde_as(as = "VecSkipError<_>")]
    pub devices: Vec<Device>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Device {
    pub name: String,
    pub tags: HashSet<String>,
    pub icon: Option<Url>,
    pub description: String,
    pub flasher: Flasher,
    pub documentation: Option<Url>,
}

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum OsListItem {
    Image(OsImage),
    SubList {
        name: String,
        description: String,
        icon: Url,
        #[serde(default)]
        flasher: Flasher,
        #[serde_as(as = "VecSkipError<_>")]
        subitems: Vec<OsListItem>,
    },
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OsImage {
    pub name: String,
    pub description: String,
    pub icon: Url,
    pub url: Url,
    #[serde(with = "const_hex")]
    pub image_download_sha256: [u8; 32],
    pub release_date: chrono::NaiveDate,
    pub devices: HashSet<String>,
    #[serde(default)]
    pub tags: HashSet<String>,
}

impl OsListItem {
    pub fn icon(&self) -> url::Url {
        match self {
            OsListItem::Image(image) => image.icon.clone(),
            OsListItem::SubList { icon, .. } => icon.clone(),
        }
    }

    pub fn search_str(&self) -> &str {
        match self {
            OsListItem::Image(os_image) => &os_image.name,
            OsListItem::SubList { name, .. } => name,
        }
    }
}

impl Config {
    pub fn from_json(data: &[u8]) -> serde_json::Result<Self> {
        serde_json::from_slice(data)
    }
}

impl From<OsImage> for crate::SelectedImage {
    fn from(value: OsImage) -> Self {
        Self::remote(value.name, value.url, value.image_download_sha256)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn basic() {
        let data = include_bytes!("../../../config.json");
        super::Config::from_json(data).unwrap();
    }
}
