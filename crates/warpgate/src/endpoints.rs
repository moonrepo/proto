use crate::error::WarpgateError;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Empty {}

// GITHUB

#[derive(Deserialize)]
pub struct GitHubApiAsset {
    pub browser_download_url: String,
    pub content_type: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct GitHubApiTag {
    pub name: String,
}

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct GitHubApiRelease {
    pub assets: Vec<GitHubApiAsset>,
}

pub async fn send_github_request<T: DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
) -> miette::Result<T> {
    let mut request = client.get(url).query(&[("per_page", "100")]);

    if let Ok(auth_token) = env::var("GITHUB_TOKEN") {
        request = request.bearer_auth(auth_token);
    }

    let handle_error = |error: reqwest::Error| WarpgateError::Http {
        error: Box::new(error),
        url: url.to_owned(),
    };

    let response = request.send().await.map_err(handle_error)?;
    let data: T = response.json().await.map_err(handle_error)?;

    Ok(data)
}
