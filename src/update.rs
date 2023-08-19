use anyhow::Result;
use reqwest::header::HeaderValue;
use semver::Version;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Release {
    pub name: String,
    pub html_url: String,
    pub tag_name: String,
    pub assets: Vec<Asset>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Asset {
    pub browser_download_url: String,
    pub name: String,
}

pub enum UpdateStatus {
    UpToDate,
    NewVersion {
        name: String,
        release_url: String,
        asset_url: Option<String>,
    },
}

#[cfg(target_os = "windows")]
fn assets_is_for_current_platform(asset: &Asset) -> bool {
    asset.name.contains("windows")
}

#[cfg(target_os = "linux")]
fn assets_is_for_current_platform(asset: &Asset) -> bool {
    asset.name.contains("linux")
}

pub fn check_update() -> Result<UpdateStatus> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("plule/vox-uristi")
        .build()?;
    let latest: Release = client
        .get("https://api.github.com/repos/plule/vox-uristi/releases/latest")
        .header(
            "Accept",
            HeaderValue::from_static("application/vnd.github+json"),
        )
        .send()?
        .json()?;

    let latest_version = Version::parse(&latest.tag_name.replace('v', ""))?;
    let current_version = Version::parse(crate::VERSION)?;

    if latest_version > current_version {
        let asset_url = latest.assets.iter().find_map(|asset| {
            if assets_is_for_current_platform(asset) {
                Some(asset.browser_download_url.clone())
            } else {
                None
            }
        });
        Ok(UpdateStatus::NewVersion {
            name: latest.name,
            release_url: latest.html_url,
            asset_url,
        })
    } else {
        Ok(UpdateStatus::UpToDate)
    }
}
