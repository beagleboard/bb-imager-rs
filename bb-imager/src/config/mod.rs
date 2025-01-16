//! Configuration for bb-imager to use.

pub mod compact;

use std::collections::{HashMap, HashSet};

use semver::Version;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, VecSkipError};
use url::Url;

use crate::common::Flasher;

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct Config {
    pub imager: Imager,
    pub os_list: Vec<OsList>,
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct Imager {
    pub latest_version: Option<Version>,
    #[serde_as(as = "VecSkipError<_>")]
    pub devices: Vec<Device>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Device {
    pub name: String,
    pub description: String,
    pub icon: Url,
    pub flasher: Flasher,
    pub documentation: Url,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OsList {
    pub name: String,
    pub description: String,
    pub icon: Url,
    pub url: Url,
    pub release_date: chrono::NaiveDate,
    #[serde(with = "const_hex")]
    pub image_sha256: [u8; 32],
    pub devices: HashSet<String>,
    pub tags: HashSet<String>,
}

impl Config {
    pub fn from_json(data: &[u8]) -> serde_json::Result<Self> {
        serde_json::from_slice(data)
    }
}

impl From<compact::Config> for Config {
    fn from(value: compact::Config) -> Self {
        let mut mapper = HashMap::new();
        let mut devices = Vec::with_capacity(value.imager.devices.len());
        let mut os_list = Vec::with_capacity(value.os_list.len());

        // Imager
        for d in value.imager.devices {
            if d.name == "No filtering" {
                continue;
            }

            let temp = d.convert(&mut mapper);
            devices.push(temp);
        }

        // OsList
        for item in value.os_list {
            let mut temp = item.convert(&mapper);
            os_list.append(&mut temp);
        }

        Self {
            imager: Imager {
                latest_version: Some(value.imager.latest_version),
                devices,
            },
            os_list,
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
