//! Add helpers to use the old bb-imager config.
//!
//! TODO: Remove if the rust version becomes the default imager.

use std::collections::{HashMap, HashSet};

use serde::Deserialize;
use url::Url;

const DEVICE_DEFAULT_ICON: &str = "https://w7.pngwing.com/pngs/132/880/png-transparent-beagleboard-beaglebone-gumstix-raspberry-pi-arm-architecture-others-miscellaneous-electronics-carnivoran-thumbnail.png";

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub(crate) imager: Imager,
    pub(crate) os_list: Vec<OsList>,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Imager {
    pub(crate) devices: Vec<Device>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Device {
    pub(crate) name: String,
    pub(crate) tags: Vec<String>,
    #[serde(default)]
    pub(crate) default: bool,
    pub(crate) icon: Option<Url>,
    pub(crate) description: String,
    pub(crate) matching_type: String,
}

impl Device {
    pub fn convert(self, mapper: &mut HashMap<String, Vec<String>>) -> super::Device {
        let icon = self
            .icon
            .unwrap_or(Url::parse(DEVICE_DEFAULT_ICON).unwrap());

        for item in self.tags {
            if let Some(x) = mapper.get_mut(&item) {
                x.push(self.name.clone());
            } else {
                mapper.insert(item, Vec::from([self.name.clone()]));
            }
        }

        super::Device {
            icon,
            flasher: super::Flasher::SdCard,
            documentation: Self::docs(&self.name),
            name: self.name,
            description: self.description,
        }
    }

    fn docs(board: &str) -> Url {
        let temp = match board {
            "BeagleY-AI" => "https://docs.beagleboard.org/boards/beagley/ai/index.html",
            "BeaglePlay" => "https://docs.beagleboard.org/boards/beagleplay/index.html",
            "BeagleBone AI-64" => "https://docs.beagleboard.org/boards/beaglebone/ai-64/index.html",
            "BeagleV-Fire" => "https://docs.beagleboard.org/boards/beaglev/fire/index.html",
            "BeagleBone Black" => "https://docs.beagleboard.org/boards/beaglebone/black/index.html",
            _ => "https://docs.beagleboard.org/",
        };

        Url::parse(temp).unwrap()
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct OsList {
    name: String,
    description: String,
    icon: Url,
    url: Option<Url>,
    extract_size: Option<u64>,
    #[serde(with = "const_hex", default)]
    extract_sha256: [u8; 32],
    image_download_size: Option<u64>,
    #[serde(with = "const_hex", default)]
    image_download_sha256: [u8; 32],
    release_date: Option<chrono::NaiveDate>,
    init_format: Option<String>,
    #[serde(default)]
    devices: Vec<String>,
    #[serde(default)]
    subitems: Vec<OsList>,
}

impl OsList {
    fn convert_item(
        self,
        mapper: &HashMap<String, Vec<String>>,
        tags: HashSet<String>,
    ) -> super::OsList {
        let devices: HashSet<String> =
            self.devices.into_iter().fold(HashSet::new(), |mut acc, t| {
                if let Some(items) = mapper.get(&t) {
                    acc.extend(items.into_iter().cloned());
                }
                acc
            });

        super::OsList {
            name: self.name,
            description: self.description,
            icon: self.icon,
            url: self.url.unwrap(),
            release_date: self.release_date.unwrap(),
            image_sha256: self.image_download_sha256,
            devices,
            tags,
        }
    }

    pub(crate) fn convert(self, mapper: &HashMap<String, Vec<String>>) -> Vec<super::OsList> {
        if self.subitems.is_empty() {
            let temp = self.convert_item(mapper, HashSet::from(["linux".to_string()]));
            Vec::from([temp])
        } else {
            let mut ans = Vec::new();

            let tags = match self.name.as_str() {
                "eMMC Flashing (other)" => HashSet::from(["linux".to_string(), "eMMC".to_string()]),
                _ => HashSet::from(["linux".to_string()]),
            };

            for item in self.subitems {
                let temp = item.convert_item(mapper, tags.clone());
                ans.push(temp);
            }
            ans
        }
    }
}
