//! Configuration for bb-imager to use.

use std::{collections::HashSet, path::PathBuf};

use semver::Version;
use serde::Deserialize;
use url::Url;

use crate::Destination;

#[derive(Deserialize, Debug, Default, Clone)]
pub struct Config {
    pub imager: Imager,
    pub os_list: Vec<OsList>,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct Imager {
    latest_version: Option<Version>,
    pub devices: Vec<Device>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Device {
    pub name: String,
    pub description: String,
    pub icon: Url,
    #[serde(with = "const_hex")]
    pub icon_sha256: [u8; 32],
    pub icon_local: Option<PathBuf>,
    pub flasher: Flasher,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OsList {
    pub name: String,
    pub description: String,
    pub icon: Url,
    #[serde(with = "const_hex")]
    pub icon_sha256: [u8; 32],
    pub icon_local: Option<PathBuf>,
    pub url: Url,
    pub release_date: chrono::NaiveDate,
    #[serde(with = "const_hex")]
    pub download_sha256: [u8; 32],
    pub extract_path: Option<String>,
    #[serde(with = "const_hex")]
    pub extracted_sha256: [u8; 32],
    pub devices: HashSet<String>,
    pub tags: HashSet<String>,
}

#[derive(Deserialize, Clone, Copy, Debug)]
pub enum Flasher {
    SdCard,
    BeagleConnectFreedom,
}

impl Config {
    pub fn from_json(data: &[u8]) -> serde_json::Result<Self> {
        serde_json::from_slice(data)
    }

    pub fn devices(&self) -> &[Device] {
        &self.imager.devices
    }

    pub fn images_by_device<'a>(
        &'a self,
        device: &'a Device,
    ) -> impl Iterator<Item = &OsList> + 'a {
        self.os_list
            .iter()
            .filter(|x| x.devices.contains(&device.name))
    }
}

impl Flasher {
    pub async fn destinations(
        &self,
        state: crate::State,
    ) -> crate::error::Result<HashSet<Destination>> {
        match self {
            Flasher::SdCard => crate::sd::destinations(&state).await,
            Flasher::BeagleConnectFreedom => crate::bcf::possible_devices().await,
        }
    }

    pub fn file_filter(&self) -> (&'static str, &'static [&'static str]) {
        match self {
            Flasher::SdCard => ("image", &["img", "xz"]),
            Flasher::BeagleConnectFreedom => ("firmware", &["bin", "zip"]),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn basic() {
        let data = include_bytes!("../../config.json");
        super::Config::from_json(data).unwrap();
    }
}
