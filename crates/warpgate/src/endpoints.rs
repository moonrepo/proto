use crate::client::HttpClient;
use crate::client_error::WarpgateClientError;
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
    client: &HttpClient,
    url: &str,
) -> miette::Result<T> {
    let mut request = client.get(url).query(&[("per_page", "100")]);

    if let Ok(auth_token) = env::var("GITHUB_TOKEN") {
        request = request.bearer_auth(auth_token);
    }

    let response = request
        .send()
        .await
        .map_err(|error| HttpClient::map_error(url.to_owned(), error))?;

    let data: T = response
        .json()
        .await
        .map_err(|error| WarpgateClientError::Http {
            error: Box::new(error),
            url: url.to_owned(),
        })?;

    Ok(data)
}
