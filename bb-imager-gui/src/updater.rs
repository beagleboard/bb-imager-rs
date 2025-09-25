//! Module to aid in providing update capabilities to the application.

use std::io;

use semver::Version;
use serde::Deserialize;

#[derive(Deserialize)]
struct GithubApiJson {
    name: String,
}

async fn latest_version(downloader: bb_downloader::Downloader) -> io::Result<Version> {
    let res: GithubApiJson = downloader
        .download_json_no_cache(crate::constants::LATEST_RELEASE_URL)
        .await?;

    let ver = res
        .name
        .strip_prefix("v")
        .ok_or(io::Error::other("Invalid version"))?;
    semver::Version::parse(ver).map_err(|e| io::Error::other(e.to_string()))
}

fn current_version() -> Version {
    semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap()
}

pub(crate) async fn check_update(
    downloader: bb_downloader::Downloader,
) -> io::Result<Option<Version>> {
    let cur_ver = current_version();
    let latest = latest_version(downloader).await?;

    if cur_ver < latest {
        Ok(Some(latest))
    } else {
        Ok(None)
    }
}
