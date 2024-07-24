//! Configuration for bb-imager to use.

use std::collections::HashSet;

use futures_core::Stream;
use semver::Version;
use serde::Deserialize;
use url::Url;

#[derive(Deserialize, Debug, Default)]
pub struct Config {
    imager: Imager,
    os_list: Vec<OsList>,
}

#[derive(Deserialize, Debug, Default)]
struct Imager {
    latest_version: Option<Version>,
    devices: Vec<Device>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Device {
    pub name: String,
    pub description: String,
    pub icon: Url,
    pub flasher: Flasher,
}

#[derive(Deserialize, Debug)]
pub struct OsList {
    pub name: String,
    pub description: String,
    pub version: Version,
    pub icon: Url,
    url: Url,
    pub release_date: chrono::NaiveDate,
    download_sha256: String,
    image_sha256: String,
    devices: HashSet<String>,
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

    pub fn images_by_device<'a>(&'a self, device: &'a Device) -> impl Iterator<Item = &OsList> + 'a {
        self.os_list.iter().filter(|x| x.devices.contains(&device.name))
    }
}

impl Flasher {
    pub fn destinations(&self) -> Result<Vec<String>, crate::bcf::BeagleConnectFreedomError> {
        match self {
            Flasher::SdCard => todo!(),
            Flasher::BeagleConnectFreedom => crate::bcf::possible_devices(),
        }
    }

    pub fn flash(
        &self,
        img: std::path::PathBuf,
        port: String,
    ) -> impl Stream<Item = Result<crate::Status, crate::bcf::BeagleConnectFreedomError>> {
        match self {
            Flasher::SdCard => todo!(),
            Flasher::BeagleConnectFreedom => crate::bcf::flash(img, port),
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
                "icon": "https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcTwA7tuf_2QUzXgjGiPx1zsCrWg03xtfn-O9Q&s",
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
            "url": "https://files.beagle.cc/file/beagleboard-public-2021/images/zephyr-microblocks-rc2.zip",
            "release_date": "2024-07-01",
            "download_sha256": "10085f9c93607843cb842580bc860151004f7f991a1166acde1d69d746c29754",
            "image_sha256": "10085f9c93607843cb842580bc860151004f7f991a1166acde1d69d746c29754",
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
