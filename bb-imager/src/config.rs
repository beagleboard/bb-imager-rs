//! Configuration for bb-imager to use.

use std::{collections::HashSet, path::PathBuf};

use futures_util::{Stream, TryStreamExt};
use semver::Version;
use serde::Deserialize;
use url::Url;

#[derive(Deserialize, Debug, Default)]
pub struct Config {
    pub imager: Imager,
    pub os_list: Vec<OsList>,
}

#[derive(Deserialize, Debug, Default)]
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
    pub icon_sha256: Vec<u8>,
    pub icon_local: Option<PathBuf>,
    pub flasher: Flasher,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OsList {
    pub name: String,
    pub description: String,
    pub version: Version,
    pub icon: Url,
    #[serde(with = "const_hex")]
    pub icon_sha256: Vec<u8>,
    pub icon_local: Option<PathBuf>,
    pub url: Url,
    pub release_date: chrono::NaiveDate,
    #[serde(with = "const_hex")]
    pub download_sha256: Vec<u8>,
    pub extract_path: String,
    #[serde(with = "const_hex")]
    pub extracted_sha256: Vec<u8>,
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
    pub fn destinations(&self) -> crate::error::Result<Vec<String>> {
        match self {
            Flasher::SdCard => todo!(),
            Flasher::BeagleConnectFreedom => crate::bcf::possible_devices().map_err(Into::into),
        }
    }

    pub fn flash(
        &self,
        img: std::path::PathBuf,
        port: String,
    ) -> impl Stream<Item = crate::error::Result<crate::FlashingStatus>> {
        match self {
            Flasher::SdCard => todo!(),
            Flasher::BeagleConnectFreedom => crate::bcf::flash(img, port).map_err(Into::into),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn basic() {
        let data = r#"
{
    "imager": {
        "latest_version": "2.0.0",
        "devices": [
            {
                "name": "BeagleConnect Freedom",
                "description": "BeagleConnect Freedom based on Ti CC1352P7",
                "icon": "https://www.mouser.in/images/marketingid/2023/img/188989252.png",
                "icon_sha256": "6b9aa96b41b90c039349572cdafcc48d648ab01fbf1f095375e9e8eac612c1db",
                "flasher": "BeagleConnectFreedom"
            }
        ]
    },
    "os_list": [
        {
            "name": "MicroBlocks",
            "description": "MicroBlocks is a blocks programming language for physical computing inspired by Scratch.",
            "version": "0.0.2",
            "icon": "https://microblocks.fun/assets/img/logos/MicroBlocks-white.svg",
            "icon_sha256": "25d1645efaa383bfb7801159a04c46e137319a37ba48f15577c4dd715d88bb04",
            "url": "https://files.beagle.cc/file/beagleboard-public-2021/images/zephyr-microblocks-rc2.zip",
            "release_date": "2024-07-01",
            "download_sha256": "10085f9c93607843cb842580bc860151004f7f991a1166acde1d69d746c29754",
            "extract_path": "zephyr/build/beagleconnect_freedom/zephyr/zephyr.bin",
            "extracted_sha256": "d6123b4159dfa4bd90e2af590a8b88782901272ff7be6e08cfd89d3099618cad",
            "devices": [
                "BeagleConnect Freedom"
            ],
            "tags": [
                "zephyr"
            ]
        }
    ]
}
            "#;

        let parsed: super::Config = serde_json::from_str(data).unwrap();
    }
}
