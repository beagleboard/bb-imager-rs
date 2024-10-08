//! Add helpers to use the old bb-imager config.
//!
//! TODO: Remove if the rust version becomes the default imager.

use std::collections::{HashMap, HashSet};

use semver::Version;
use serde::Deserialize;
use sha2::Digest;
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
    pub async fn convert(
        self,
        client: &reqwest::Client,
        mapper: &mut HashMap<String, Vec<String>>,
    ) -> super::Device {
        let icon = self
            .icon
            .unwrap_or(Url::parse(DEVICE_DEFAULT_ICON).unwrap());
        let icon_sha256 = icon_sha256(client, icon.clone()).await;

        for item in self.tags {
            if let Some(x) = mapper.get_mut(&item) {
                x.push(self.name.clone());
            } else {
                mapper.insert(item, Vec::from([self.name.clone()]));
            }
        }

        super::Device {
            icon,
            icon_sha256,
            flasher: super::Flasher::SdCard,
            documentation: Self::docs(&self.name),
            name: self.name,
            description: self.description,
        }
    }

    fn docs(board: &str) -> Url {
        match board {
            _ => Url::parse("https://docs.beagleboard.org/").unwrap(),
        }
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
    async fn convert_item(
        self,
        client: &reqwest::Client,
        mapper: &HashMap<String, Vec<String>>,
        tags: HashSet<String>,
    ) -> super::OsList {
        let icon_sha256 = icon_sha256(client, self.icon.clone()).await;
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
            icon_sha256,
            url: self.url.unwrap(),
            release_date: self.release_date.unwrap(),
            extract_sha256: self.extract_sha256,
            extract_path: None,
            devices,
            tags,
        }
    }

    pub(crate) async fn convert(
        self,
        client: &reqwest::Client,
        mapper: &HashMap<String, Vec<String>>,
    ) -> Vec<super::OsList> {
        if self.subitems.is_empty() {
            let temp = self
                .convert_item(client, mapper, HashSet::from(["linux".to_string()]))
                .await;
            Vec::from([temp])
        } else {
            let mut ans = Vec::new();

            let tags = match self.name.as_str() {
                "eMMC Flashing (other)" => HashSet::from(["linux".to_string(), "eMMC".to_string()]),
                _ => HashSet::from(["linux".to_string()]),
            };

            for item in self.subitems {
                let temp = item.convert_item(client, mapper, tags.clone()).await;
                ans.push(temp);
            }
            ans
        }
    }
}

async fn icon_sha256(client: &reqwest::Client, icon: Url) -> [u8; 32] {
    let icon_data = client
        .get(icon)
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();

    sha2::Sha256::new()
        .chain_update(&icon_data)
        .finalize()
        .try_into()
        .unwrap()
}
