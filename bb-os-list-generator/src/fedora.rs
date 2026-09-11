use bb_config::config::{OsImage, OsListItem, OsSubList};
use chrono::DateTime;
use reqwest::header::LAST_MODIFIED;
use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};
use tokio::task::JoinSet;
use url::Url;

const ICON: &str = "https://upload.wikimedia.org/wikipedia/commons/4/41/Fedora_icon_%282021%29.svg?utm_source=commons.wikimedia.org&utm_campaign=index&utm_content=original";
const RELEASE_JSON: &str = "https://fedoraproject.org/releases.json";

#[serde_as]
#[derive(Deserialize, Debug)]
struct FedoraItem {
    #[serde_as(as = "DisplayFromStr")]
    version: u8,
    arch: Box<str>,
    link: Url,
    variant: Box<str>,
    subvariant: Box<str>,
    #[serde(with = "const_hex")]
    sha256: [u8; 32],
    #[serde_as(as = "DisplayFromStr")]
    size: u64,
}

impl FedoraItem {
    async fn into_os_image(self, client: reqwest::Client) -> OsImage {
        let header = client.head(self.link.clone()).send().await.unwrap();

        let release_date = header
            .headers()
            .get(LAST_MODIFIED)
            .unwrap()
            .to_str()
            .unwrap();
        let release_date = DateTime::parse_from_rfc2822(release_date).unwrap();

        let extract_size = crate::helpers::xz_extract_size(&client, &self.link, self.size)
            .await
            .unwrap();

        OsImage {
            name: format!("Fedora {} {}", self.version, self.subvariant),
            description: self.description().unwrap().into(),
            icon: Url::parse(ICON).unwrap(),
            url: self.link,
            image_download_size: Some(self.size),
            image_download_sha256: self.sha256,
            extract_size,
            release_date: release_date.date_naive(),
            devices: ["beagle-am67".into()].into(),
            init_format: bb_config::config::InitFormat::None,
            bmap: None,
            info_text: None,
            support: None,
        }
    }

    fn description(&self) -> Option<&'static str> {
        match self.subvariant.as_ref() {
            "Minimal" => Some("The smallest possible Fedora installation; no desktop environment"),
            _ => None,
        }
    }
}

pub(crate) async fn os_list_items(client: reqwest::Client) -> Vec<OsListItem> {
    let data: Vec<FedoraItem> = client
        .get(RELEASE_JSON)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let latest_version = data.iter().map(|x| x.version).max().unwrap();

    let tasks: JoinSet<_> = data
        .into_iter()
        // Only get the latest 2 versions.
        .filter(|x| x.version >= latest_version - 1)
        .filter(|x| x.arch.as_ref() == "aarch64")
        .filter(|x| x.variant.as_ref() == "Spins" && x.subvariant.as_ref() == "Minimal")
        .map(|x| x.into_os_image(client.clone()))
        .collect();

    let imgs: Vec<_> = tasks
        .join_all()
        .await
        .into_iter()
        .map(OsListItem::Image)
        .collect();

    [OsListItem::SubList(OsSubList {
        name: "Fedora Images".into(),
        description: "Fedora Linux is an innovative platform for hardware, clouds, and containers, built with love by you."
            .into(),
        icon: Url::parse(ICON).unwrap(),
        flasher: bb_config::config::Flasher::SdCardNoBootloader,
        subitems: imgs,
    })]
    .into()
}
