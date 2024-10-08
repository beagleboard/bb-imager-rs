//! Configuration for bb-imager to use.

pub mod compact;

use std::collections::{HashMap, HashSet};

use semver::Version;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    flasher::{bcf, msp430, sd},
    Destination,
};

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct Config {
    pub imager: Imager,
    pub os_list: Vec<OsList>,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct Imager {
    pub latest_version: Option<Version>,
    pub devices: Vec<Device>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Device {
    pub name: String,
    pub description: String,
    pub icon: Url,
    #[serde(with = "const_hex")]
    pub icon_sha256: [u8; 32],
    pub flasher: Flasher,
    pub documentation: Url,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OsList {
    pub name: String,
    pub description: String,
    pub icon: Url,
    #[serde(with = "const_hex")]
    pub icon_sha256: [u8; 32],
    pub url: Url,
    pub release_date: chrono::NaiveDate,
    #[serde(with = "const_hex")]
    pub extract_sha256: [u8; 32],
    pub extract_path: Option<String>,
    pub devices: HashSet<String>,
    pub tags: HashSet<String>,
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
pub enum Flasher {
    SdCard,
    BeagleConnectFreedom,
    Msp430Usb,
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
    ) -> impl Iterator<Item = &'a OsList> + 'a {
        self.os_list
            .iter()
            .filter(|x| x.devices.contains(&device.name))
    }

    pub async fn merge_compact(mut self, comp: compact::Config, client: reqwest::Client) -> Self {
        let mut mapper = HashMap::new();

        // Imager
        self.imager.devices.reserve(comp.imager.devices.len());
        for d in comp.imager.devices {
            if d.name == "No filtering" {
                continue;
            }

            let temp = d.convert(&client, &mut mapper).await;
            self.imager.devices.push(temp);
        }

        // OsList
        self.os_list.reserve(comp.os_list.len());
        for item in comp.os_list {
            let mut temp = item.convert(&client, &mapper).await;
            self.os_list.append(&mut temp);
        }

        self
    }
}

impl Flasher {
    pub async fn destinations(&self) -> HashSet<Destination> {
        match self {
            Flasher::SdCard => tokio::task::block_in_place(sd::destinations),
            Flasher::BeagleConnectFreedom => tokio::task::block_in_place(bcf::possible_devices),
            Flasher::Msp430Usb => tokio::task::block_in_place(msp430::possible_devices),
        }
    }

    pub fn file_filter(&self) -> (&'static str, &'static [&'static str]) {
        match self {
            Flasher::SdCard => ("image", &["img", "xz"]),
            Flasher::BeagleConnectFreedom => ("firmware", &["bin", "hex", "txt", "xz"]),
            Flasher::Msp430Usb => ("firmware", &["hex", "txt", "xz"]),
        }
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
