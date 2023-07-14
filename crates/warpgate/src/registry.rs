use crate::api::*;
use crate::error::WarpgateError;
use crate::helpers::extract_prefix_from_slug;
use crate::locator::{GitHubLocator, PluginLocator, WapmLocator};
use miette::IntoDiagnostic;
use sha2::{Digest, Sha256};
use starbase_styles::color;
use std::path::{Path, PathBuf};
use tracing::trace;

pub struct PluginRegistry {
    /// Location where downloaded .wasm plugins are stored.
    plugins_dir: PathBuf,

    // Location where temporary files (like archives) are stored.
    temp_dir: PathBuf,
}

impl PluginRegistry {
    pub fn new(plugins_dir: &Path, temp_dir: &Path) -> Self {
        trace!(cache_dir = ?plugins_dir, "Creating plugin registry");

        Self {
            plugins_dir: plugins_dir.to_owned(),
            temp_dir: temp_dir.to_owned(),
        }
    }

    pub async fn load_plugin<T: AsRef<str>>(
        &mut self,
        name: T,
        locator: &PluginLocator,
    ) -> miette::Result<PathBuf> {
        let name = name.as_ref();

        trace!(
            plugin = name,
            locator = locator.to_string(),
            "Loading plugin {}",
            color::id(name)
        );

        match locator {
            PluginLocator::SourceFile { path, .. } => {
                let path = path
                    .canonicalize()
                    .map_err(|_| WarpgateError::SourceFileMissing(path.to_path_buf()))?;

                if path.exists() {
                    trace!(
                        plugin = name,
                        path = ?path,
                        "Using source file",
                    );

                    Ok(path)
                } else {
                    Err(WarpgateError::SourceFileMissing(path).into())
                }
            }
            PluginLocator::SourceUrl { url } => {
                self.download_plugin(name, url, self.create_cache_path(name, url))
                    .await
            }
            PluginLocator::GitHub(github) => self.download_plugin_from_github(name, github).await,
            PluginLocator::Wapm(wapm) => self.download_plugin_from_wapm(name, wapm).await,
        }
    }

    fn create_cache_path(&self, name: &str, seed: &str) -> PathBuf {
        let mut sha = Sha256::new();
        sha.update(seed);

        self.plugins_dir
            .join(format!("{}-{:x}.wasm", name, sha.finalize()))
    }

    async fn download_plugin(
        &mut self,
        name: &str,
        source_url: &str,
        dest_path: PathBuf,
    ) -> miette::Result<PathBuf> {
        if dest_path.exists() {
            return Ok(dest_path);
        }

        trace!(
            plugin = name,
            url = source_url,
            "Downloading plugin from URL",
        );

        Ok(dest_path)
    }

    async fn download_plugin_from_github(
        &mut self,
        name: &str,
        github: &GitHubLocator,
    ) -> miette::Result<PathBuf> {
        let (api_url, release_tag) = if let Some(tag) = &github.tag {
            (
                format!(
                    "https://api.github.com/repos/{}/releases/tags/{tag}",
                    github.repo_slug,
                ),
                tag.to_owned(),
            )
        } else {
            (
                format!(
                    "https://api.github.com/repos/{}/releases/latest",
                    github.repo_slug,
                ),
                "latest".to_owned(),
            )
        };

        trace!(
            plugin = name,
            api_url = &api_url,
            release_tag = &release_tag,
            "Attempting to download plugin from GitHub release",
        );

        // Check the cache first using the API URL as the seed,
        // so that we can avoid making unnecessary HTTP requests.
        let plugin_path = self.create_cache_path(name, &api_url);

        if plugin_path.exists() {
            return Ok(plugin_path);
        }

        // Otherwise make an HTTP request to the GitHub releases API,
        // and loop through the assets to find a matching one.
        let response = reqwest::get(api_url).await.into_diagnostic()?;
        let release: GitHubApiRelease = response.json().await.into_diagnostic()?;

        // Find a direct WASM asset first
        for asset in &release.assets {
            if asset.content_type == "application/wasm" || asset.name.ends_with(".wasm") {
                trace!(
                    plugin = name,
                    asset = &asset.name,
                    "Found WASM asset with `application/wasm` content type"
                );

                return self
                    .download_plugin(name, &asset.browser_download_url, plugin_path)
                    .await;
            }
        }

        // Otherwise an asset with a matching name and supported extension
        for asset in release.assets {
            if asset.name == github.file_stem
                || (asset.name.starts_with(&github.file_stem)
                    && (asset.name.ends_with(".tar")
                        | asset.name.ends_with(".tar.gz")
                        | asset.name.ends_with(".tgz")
                        | asset.name.ends_with(".tar.xz")
                        | asset.name.ends_with(".txz")
                        | asset.name.ends_with(".zip")))
            {
                trace!(
                    plugin = name,
                    asset = &asset.name,
                    "Found possible asset as an archive"
                );

                return self
                    .download_plugin(name, &asset.browser_download_url, plugin_path)
                    .await;
            }
        }

        Err(WarpgateError::GitHubAssetMissing {
            repo_slug: github.repo_slug.to_owned(),
            tag: release_tag,
        }
        .into())
    }

    async fn download_plugin_from_wapm(
        &mut self,
        name: &str,
        wapm: &WapmLocator,
    ) -> miette::Result<PathBuf> {
        let version = wapm.version.as_deref().unwrap_or("latest");
        let fake_api_url = format!(
            "https://registry.wapm.io/graphql/{}@{}",
            wapm.package_name, version
        );

        trace!(
            plugin = name,
            api_url = &fake_api_url,
            version,
            "Attempting to download plugin from wamp.io",
        );

        // Check the cache first using the API URL as the seed,
        // so that we can avoid making unnecessary HTTP requests.
        let plugin_path = self.create_cache_path(name, &fake_api_url);

        if plugin_path.exists() {
            return Ok(plugin_path);
        }

        // Otherwise make a GraphQL request to the WAPM registry API.
        let client = reqwest::Client::new();
        let response = client
            .post("https://registry.wapm.io/graphql")
            .json(&WapmPackageRequest {
                query: WAPM_GQL_QUERY.to_owned(),
                variables: WapmPackageRequestVariables {
                    name: wapm.package_name.to_owned(),
                    owner: extract_prefix_from_slug(&wapm.package_name).to_owned(),
                    version: version.to_owned(),
                },
            })
            .send()
            .await
            .into_diagnostic()?;

        let package: WapmPackageResponse = response.json().await.into_diagnostic()?;
        let package = package.data.package_version;

        // Check modules first for a direct WASM file to use
        let modules = package
            .modules
            .iter()
            .filter(|module| module.abi == "wasi" && module.public_url.ends_with(".wasm"))
            .collect::<Vec<_>>();

        if let Some(release_module) = modules.iter().find(|module| {
            module
                .public_url
                .ends_with(&format!("release/{}.wasm", wapm.file_stem))
        }) {
            trace!(
                plugin = name,
                module = &release_module.name,
                "Found possible module compiled for release mode"
            );

            return self
                .download_plugin(name, &release_module.public_url, plugin_path)
                .await;
        }

        if let Some(fallback_module) = modules.iter().find(|module| {
            module.name == wapm.file_stem || module.name == format!("{}.wasm", wapm.file_stem)
        }) {
            trace!(
                plugin = name,
                module = &fallback_module.name,
                "Found possible module with matching file name"
            );

            return self
                .download_plugin(name, &fallback_module.public_url, plugin_path)
                .await;
        }

        // Otherwise use the distribution download, which is typically an archive
        if let Some(download_url) = &package.distribution.download_url {
            trace!(
                plugin = name,
                "Using the distribution archive as a last resort"
            );

            return self.download_plugin(name, download_url, plugin_path).await;
        }

        Err(WarpgateError::WapmModuleMissing {
            package: wapm.package_name.to_owned(),
            version: version.to_owned(),
        }
        .into())
    }
}
