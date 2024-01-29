use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Empty {}

// GITHUB

#[derive(Deserialize)]
pub struct GitHubApiAsset {
    pub browser_download_url: String,
    pub content_type: String,
    pub name: String,
}

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct GitHubApiRelease {
    pub assets: Vec<GitHubApiAsset>,
}
