use bb_config::config::{OsListItem, OsSubList};
use serde::Deserialize;
use serde_with::serde_as;
use serde_with::{DisplayFromStr, NoneAsEmptyString};
use tokio::task::JoinSet;
use url::Url;

const ARMBIAN_IMAGE_LIST: &str = "https://github.armbian.com/armbian-images.json";
const ARMBIAN_ICON: &str = "https://www.beagleboard.org/app/uploads/2025/04/armbian-logo.jpg";

#[derive(Deserialize, Debug)]
struct ArmbianList {
    assets: Vec<ArbianImage>,
}

#[serde_as]
#[derive(Deserialize, Debug)]
struct ArbianImage {
    board_slug: Box<str>,
    board_name: Box<str>,
    board_vendor: Box<str>,
    file_url: Url,
    file_url_sha: Url,
    armbian_version: Box<str>,
    #[serde_as(as = "DisplayFromStr")]
    file_size: u64,
    distro: Box<str>,
    variant: Box<str>,
    kernel_version: Box<str>,
    /// Upstream uses an empty string, not `null`, when an image is not an application build.
    #[serde_as(as = "NoneAsEmptyString")]
    file_application: Option<String>,
    file_date: chrono::DateTime<chrono::Utc>,
}

impl ArbianImage {
    async fn into_os_image(
        self,
        downloader: reqwest::Client,
    ) -> Option<bb_config::config::OsImage> {
        let dev = self.device()?;
        let sha256_file = downloader
            .get(self.file_url_sha.clone())
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        let sha256 = sha256_file.split_whitespace().next().unwrap().trim();

        let extract_size =
            crate::helpers::xz_extract_size(&downloader, &self.file_url, self.file_size)
                .await
                .unwrap();

        Some(bb_config::config::OsImage {
            name: self.name(),
            description: self.description(),
            icon: Url::parse(ARMBIAN_ICON).unwrap(),
            url: self.file_url,
            image_download_size: Some(self.file_size),
            image_download_sha256: const_hex::decode(sha256).unwrap().try_into().unwrap(),
            extract_size,
            release_date: self.file_date.date_naive(),
            devices: Box::new([dev.into()]),
            init_format: bb_config::config::InitFormat::Armbian,
            bmap: None,
            info_text: None,
            support: None,
        })
    }

    /// The only text shown in the image list, so it has to carry every token that separates
    /// one build of a board from another.
    fn name(&self) -> String {
        let label = self.application_name().unwrap_or(self.variant_name());

        format!(
            "{} Armbian {} v{} {}",
            self.board_name,
            self.distro_name().unwrap(),
            self.kernel_version,
            label
        )
    }

    fn description(&self) -> String {
        let distro = self.distro_name().unwrap();

        // Release date and sizes already get their own rows in the GUI, so leave them out.
        let built = format!(
            "for {}, built by Armbian {} kernel {}.",
            self.board_name, self.armbian_version, self.kernel_version
        );

        match self.application_name() {
            Some(app) => format!("{} on {} {}", app, distro, built),
            None if self.is_headless() => {
                format!("{distro} with no desktop environment {built}")
            }
            None => format!(
                "{} with the {} desktop {}",
                distro,
                self.variant_name(),
                built
            ),
        }
    }

    /// Variants that ship without a desktop environment.
    fn is_headless(&self) -> bool {
        matches!(self.variant.as_ref(), "minimal" | "server")
    }

    /// Release a distro codename belongs to. Unknown codenames fall back to the capitalized
    /// codename so a newly branched Debian/Ubuntu release still reads sensibly.
    fn distro_name(&self) -> Option<&'static str> {
        match self.distro.as_ref() {
            "bookworm" => Some("Debian 12 (bookworm)"),
            "trixie" => Some("Debian 13 (trixie)"),
            "forky" => Some("Debian 14 (forky)"),
            "sid" => Some("Debian Sid"),
            "jammy" => Some("Ubuntu 22.04 (jammy)"),
            "noble" => Some("Ubuntu 24.04 (noble)"),
            "oracular" => Some("Ubuntu 24.10 (oracular)"),
            "plucky" => Some("Ubuntu 25.04 (plucky)"),
            "questing" => Some("Ubuntu 25.10 (questing)"),
            "resolute" => Some("Ubuntu 26.04 (resolute)"),
            _ => None,
        }
    }

    fn variant_name(&self) -> &str {
        match self.variant.as_ref() {
            "minimal" => "Minimal",
            "server" => "Server",
            "gnome" => "GNOME",
            "xfce" => "Xfce",
            "kde-plasma" => "KDE Plasma",
            "kde-neon" => "KDE Neon",
            "cinnamon" => "Cinnamon",
            "mate" => "MATE",
            "i3-wm" => "i3",
            x => x,
        }
    }

    fn application_name(&self) -> Option<&str> {
        self.file_application.as_deref().map(|app| match app {
            "kali" => "Kali Linux",
            "homeassistant" => "Home Assistant",
            "omv" => "OpenMediaVault",
            "sdk" => "SDK",
            x => x,
        })
    }

    fn device(&self) -> Option<&'static str> {
        match self.board_slug.as_ref() {
            "pocketbeagle2" => Some("pocketbeagle2-am62"),
            "beaglebone-ai64" => Some("beagle-tda4vm"),
            "beagleplay" => Some("beagle-am62"),
            "beagley-ai" => Some("beagle-am67"),
            _ => None,
        }
    }
}

pub(crate) async fn os_list_items(client: reqwest::Client) -> Vec<OsListItem> {
    let data: ArmbianList = client
        .get(ARMBIAN_IMAGE_LIST)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let tasks: JoinSet<_> = data
        .assets
        .into_iter()
        .filter(|x| x.board_vendor.as_ref() == "beagleboard")
        .map(|x| x.into_os_image(client.clone()))
        .collect();

    let imgs: Vec<_> = tasks
        .join_all()
        .await
        .into_iter()
        .flatten()
        .map(OsListItem::Image)
        .collect();

    [OsListItem::SubList(OsSubList {
        name: "Armbian Images (rolling)".into(),
        description: "Rolling Debian and Ubuntu builds from the Armbian project. Community \
                      maintained and lightly tested"
            .into(),
        icon: Url::parse(ARMBIAN_ICON).unwrap(),
        flasher: bb_config::config::Flasher::SdCard,
        subitems: imgs,
    })]
    .into()
}
