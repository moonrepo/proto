use super::{LoadFrom, LoaderProtocol};
use crate::clients::{HttpClient, WarpgateHttpClientError};
use crate::loader_error::WarpgateLoaderError;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use starbase_archive::is_supported_archive_extension;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::trace;
use warpgate_api::{GitHubLocator, Id};

#[derive(Clone)]
pub struct GitHubLoader {
    pub client: Arc<HttpClient>,
}

impl GitHubLoader {
    async fn request_api<T: DeserializeOwned>(
        &self,
        url: &str,
    ) -> Result<T, WarpgateHttpClientError> {
        let mut request = self.client.get(url).query(&[("per_page", "100")]);

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
            .map_err(|error| WarpgateHttpClientError::Http {
                error: Box::new(error),
                url: url.to_owned(),
            })?;

        Ok(data)
    }
}

impl LoaderProtocol<GitHubLocator> for GitHubLoader {
    type Data = ();

    fn is_latest(&self, locator: &GitHubLocator) -> bool {
        locator.tag.as_ref().is_some_and(|tag| tag == "latest")
            || locator.tag.is_none() && locator.project_name.is_none()
    }

    async fn load(
        &self,
        id: &Id,
        locator: &GitHubLocator,
        _: &Self::Data,
    ) -> Result<LoadFrom, WarpgateLoaderError> {
        // Fetch all tags to find a matching tag + release
        let tags_url = format!("https://api.github.com/repos/{}/tags", locator.repo_slug);
        let found_tag;

        trace!(
            id = id.as_str(),
            tag = locator.tag.as_ref(),
            tag_prefix = locator.project_name.as_ref(),
            tags_url = &tags_url,
            "Attempting to find a matching Git tag",
        );

        if let Some(tag) = &locator.tag {
            found_tag = Some(tag.to_owned())
        } else if let Some(tag_prefix) = &locator.project_name {
            found_tag = self
                .request_api::<Vec<GitHubApiTag>>(&tags_url)
                .await?
                .into_iter()
                .find(|row| {
                    row.name.starts_with(format!("{tag_prefix}@").as_str())
                        || row.name.starts_with(format!("{tag_prefix}-").as_str())
                })
                .map(|row| row.name);
        } else {
            found_tag = Some("latest".into());
        }

        let Some(release_tag) = found_tag else {
            return Err(WarpgateLoaderError::MissingGitHubTag {
                id: id.to_owned(),
                repo_slug: locator.repo_slug.to_owned(),
            });
        };

        // Then find a release with assets
        let release_url = if release_tag == "latest" {
            format!(
                "https://api.github.com/repos/{}/releases/latest",
                locator.repo_slug,
            )
        } else {
            format!(
                "https://api.github.com/repos/{}/releases/tags/{release_tag}",
                locator.repo_slug,
            )
        };

        trace!(
            id = id.as_str(),
            release_url = &release_url,
            release_tag = &release_tag,
            "Attempting to download plugin from GitHub release",
        );

        let release: GitHubApiRelease = self.request_api(&release_url).await?;

        // Find a direct WASM asset first
        for asset in &release.assets {
            if asset.content_type == "application/wasm" || asset.name.ends_with(".wasm") {
                trace!(
                    id = id.as_str(),
                    asset = &asset.name,
                    "Found WASM asset with application/wasm content type"
                );

                return Ok(LoadFrom::Url(asset.browser_download_url.clone()));
            }
        }

        // Otherwise an asset with a matching name and supported extension
        if let Some(tag_prefix) = &locator.project_name {
            for asset in release.assets {
                if &asset.name == tag_prefix
                    || (asset.name.starts_with(tag_prefix)
                        && is_supported_archive_extension(&PathBuf::from(&asset.name)))
                {
                    trace!(
                        id = id.as_str(),
                        asset = &asset.name,
                        "Found possible asset as an archive"
                    );

                    return Ok(LoadFrom::Url(asset.browser_download_url.clone()));
                }
            }
        }

        Err(WarpgateLoaderError::MissingGitHubAsset {
            id: id.to_owned(),
            repo_slug: locator.repo_slug.to_owned(),
            tag: release_tag,
        })
    }
}

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
