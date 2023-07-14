use crate::api::*;
use crate::error::WarpgateError;
use crate::locator::{GitHubLocator, PluginLocator, WapmLocator};
use miette::IntoDiagnostic;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub struct PluginRegistry {
    /// Location where downloaded plugins are stored.
    plugins_dir: PathBuf,
}

impl PluginRegistry {
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self { plugins_dir }
    }

    pub async fn load_plugin<T: AsRef<str>>(
        &mut self,
        name: T,
        locator: &PluginLocator,
    ) -> miette::Result<PathBuf> {
        let name = name.as_ref();

        match locator {
            PluginLocator::SourceFile { path, .. } => {
                let path = path
                    .canonicalize()
                    .map_err(|_| WarpgateError::SourceFileMissing(path.to_path_buf()))?;

                if path.exists() {
                    Ok(path)
                } else {
                    Err(WarpgateError::SourceFileMissing(path).into())
                }
            }
            PluginLocator::SourceUrl { url } => {
                self.download_plugin(&url, self.create_cache_path(name, &url))
                    .await
            }
            PluginLocator::GitHub(github) => self.download_plugin_from_github(name, &github).await,
            PluginLocator::Wapm(wapm) => self.download_plugin_from_wapm(name, &wapm).await,
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
        source_url: &str,
        dest_path: PathBuf,
    ) -> miette::Result<PathBuf> {
        if dest_path.exists() {
            return Ok(dest_path);
        }

        Ok(dest_path)
    }

    async fn download_plugin_from_github(
        &mut self,
        name: &str,
        github: &GitHubLocator,
    ) -> miette::Result<PathBuf> {
        let (api_url, release_tag) = if let Some(version) = &github.version {
            (
                format!(
                    "https://api.github.com/repos/{}/releases/tags/v{version}",
                    github.repo_slug,
                ),
                format!("v{version}"),
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
                return self
                    .download_plugin(&asset.browser_download_url, plugin_path)
                    .await;
            }
        }

        // Otherwise an asset with a matching name and supported extension
        for asset in release.assets {
            if asset.name == github.asset_name
                || (asset.name.starts_with(&github.asset_name)
                    && (asset.name.ends_with(".tar")
                        | asset.name.ends_with(".tar.gz")
                        | asset.name.ends_with(".tgz")
                        | asset.name.ends_with(".tar.xz")
                        | asset.name.ends_with(".txz")
                        | asset.name.ends_with(".zip")))
            {
                return self
                    .download_plugin(&asset.browser_download_url, plugin_path)
                    .await;
            }
        }

        Err(WarpgateError::GitHubAssetMissing {
            repo_slug: github.repo_slug.to_owned(),
            release: release_tag,
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
                    owner: extract_owner_from_package(&wapm.package_name).to_owned(),
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
                .ends_with(&format!("release/{}.wasm", wapm.wasm_name))
        }) {
            return self
                .download_plugin(&release_module.public_url, plugin_path)
                .await;
        }

        if let Some(fallback_module) = modules.iter().find(|module| {
            module.name == wapm.wasm_name || module.name == format!("{}.wasm", wapm.wasm_name)
        }) {
            return self
                .download_plugin(&fallback_module.public_url, plugin_path)
                .await;
        }

        // Otherwise use the distribution download, which is typically an archive
        if let Some(download_url) = &package.distribution.download_url {
            return self.download_plugin(download_url, plugin_path).await;
        }

        Err(WarpgateError::WapmModuleMissing {
            package: wapm.package_name.to_owned(),
            release: version.to_owned(),
        }
        .into())
    }
}

fn extract_owner_from_package(package: &str) -> &str {
    package
        .split('/')
        .next()
        .expect("Expected package to have an owner scope!")
}
