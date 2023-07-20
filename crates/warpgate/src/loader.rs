use crate::api::*;
use crate::error::WarpgateError;
use crate::helpers::{
    determine_cache_extension, download_url_to_temp, extract_prefix_from_slug,
    move_or_unpack_download,
};
use crate::locator::{GitHubLocator, PluginLocator, WapmLocator};
use sha2::{Digest, Sha256};
use starbase_styles::color;
use starbase_utils::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tracing::trace;

pub struct PluginLoader {
    /// Location where downloaded .wasm plugins are stored.
    plugins_dir: PathBuf,

    /// Location where temporary files (like archives) are stored.
    temp_dir: PathBuf,
}

impl PluginLoader {
    /// Create a new loader that stores plugins and downloads in the provided directories.
    pub fn new<P: AsRef<Path>, T: AsRef<Path>>(plugins_dir: P, temp_dir: T) -> Self {
        let plugins_dir = plugins_dir.as_ref();

        trace!(cache_dir = ?plugins_dir, "Creating plugin loader");

        Self {
            plugins_dir: plugins_dir.to_owned(),
            temp_dir: temp_dir.as_ref().to_owned(),
        }
    }

    /// Load a plugin using the provided locator. File system plugins are loaded directly,
    /// while remote/URL plugins are downloaded and cached.
    pub async fn load_plugin<T: AsRef<str>>(
        &mut self,
        id: T,
        locator: &PluginLocator,
    ) -> miette::Result<PathBuf> {
        let id = id.as_ref();

        trace!(
            plugin = id,
            locator = locator.to_string(),
            "Loading plugin {}",
            color::id(id)
        );

        match locator {
            PluginLocator::SourceFile { path, .. } => {
                let path = path
                    .canonicalize()
                    .map_err(|_| WarpgateError::SourceFileMissing(path.to_path_buf()))?;

                if path.exists() {
                    trace!(
                        plugin = id,
                        path = ?path,
                        "Using source file",
                    );

                    Ok(path)
                } else {
                    Err(WarpgateError::SourceFileMissing(path).into())
                }
            }
            PluginLocator::SourceUrl { url } => {
                self.download_plugin(
                    id,
                    url,
                    self.create_cache_path(id, url, url.contains("latest")),
                )
                .await
            }
            PluginLocator::GitHub(github) => self.download_plugin_from_github(id, github).await,
            PluginLocator::Wapm(wapm) => self.download_plugin_from_wapm(id, wapm).await,
        }
    }

    /// Create an absolute path to the plugin's destination file, located in the plugins directory.
    /// Hash the source URL to ensure uniqueness of each plugin + version combination.
    fn create_cache_path(&self, id: &str, url: &str, is_latest: bool) -> PathBuf {
        let mut sha = Sha256::new();
        sha.update(url);

        self.plugins_dir.join(format!(
            "{id}{}{:x}{}",
            if is_latest { "-latest-" } else { "-" },
            sha.finalize(),
            determine_cache_extension(url)
        ))
    }

    /// Check if the plugin has been downloaded and is cached.
    /// If using a latest strategy (no explicit version or tag), the cache
    /// is only valid for 7 days (to ensure not stale), otherwise forever.
    fn is_cached(&self, id: &str, path: &Path) -> miette::Result<bool> {
        if !path.exists() {
            trace!(plugin = id, "Plugin not cached, downloading");

            return Ok(false);
        }

        let mut cached = true;

        // If latest, cache only lasts for 7 days
        if fs::file_name(path).contains("-latest-") {
            let metadata = fs::metadata(path)?;

            cached = if let Ok(filetime) = metadata.created().or_else(|_| metadata.modified()) {
                filetime > SystemTime::now() - Duration::from_secs(86400 * 7)
            } else {
                false
            };

            if !cached {
                trace!(plugin = id, path = ?path, "Deleting stale cache");

                fs::remove_file(path)?;
            }
        }

        if cached {
            trace!(plugin = id, path = ?path, "Plugin already downloaded and cached");
        } else {
            trace!(plugin = id, path = ?path, "Plugin cached but stale, re-downloading");
        }

        Ok(cached)
    }

    async fn download_plugin(
        &mut self,
        id: &str,
        source_url: &str,
        dest_path: PathBuf,
    ) -> miette::Result<PathBuf> {
        if self.is_cached(id, &dest_path)? {
            return Ok(dest_path);
        }

        trace!(plugin = id, url = source_url, "Downloading plugin from URL");

        move_or_unpack_download(
            &download_url_to_temp(source_url, &self.temp_dir).await?,
            &dest_path,
        )?;

        Ok(dest_path)
    }

    async fn download_plugin_from_github(
        &mut self,
        id: &str,
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

        // Check the cache first using the API URL as the seed,
        // so that we can avoid making unnecessary HTTP requests.
        let plugin_path = self.create_cache_path(id, &api_url, release_tag == "latest");

        if self.is_cached(id, &plugin_path)? {
            return Ok(plugin_path);
        }

        trace!(
            plugin = id,
            api_url = &api_url,
            release_tag = &release_tag,
            "Attempting to download plugin from GitHub release",
        );

        // Otherwise make an HTTP request to the GitHub releases API,
        // and loop through the assets to find a matching one.
        let client = reqwest::Client::new();
        let response = client
            .get(api_url)
            .header("User-Agent", "moonrepo/proto")
            .send()
            .await
            .map_err(|error| WarpgateError::Http { error })?;
        let release: GitHubApiRelease = response
            .json()
            .await
            .map_err(|error| WarpgateError::Http { error })?;

        // Find a direct WASM asset first
        for asset in &release.assets {
            if asset.content_type == "application/wasm" || asset.name.ends_with(".wasm") {
                trace!(
                    plugin = id,
                    asset = &asset.name,
                    "Found WASM asset with application/wasm content type"
                );

                return self
                    .download_plugin(id, &asset.browser_download_url, plugin_path)
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
                    plugin = id,
                    asset = &asset.name,
                    "Found possible asset as an archive"
                );

                return self
                    .download_plugin(id, &asset.browser_download_url, plugin_path)
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
        id: &str,
        wapm: &WapmLocator,
    ) -> miette::Result<PathBuf> {
        let version = wapm.version.as_deref().unwrap_or("latest");
        let fake_api_url = format!(
            "https://registry.wapm.io/graphql/{}@{}",
            wapm.package_name, version
        );

        // Check the cache first using the API URL as the seed,
        // so that we can avoid making unnecessary HTTP requests.
        let plugin_path = self.create_cache_path(id, &fake_api_url, version == "latest");

        if self.is_cached(id, &plugin_path)? {
            return Ok(plugin_path);
        }

        trace!(
            plugin = id,
            api_url = &fake_api_url,
            version,
            "Attempting to download plugin from wamp.io",
        );

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
            .map_err(|error| WarpgateError::Http { error })?;

        let package: WapmPackageResponse = response
            .json()
            .await
            .map_err(|error| WarpgateError::Http { error })?;
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
                plugin = id,
                module = &release_module.name,
                "Found possible module compiled for release mode"
            );

            return self
                .download_plugin(id, &release_module.public_url, plugin_path)
                .await;
        }

        if let Some(fallback_module) = modules.iter().find(|module| {
            module.name == wapm.file_stem || module.name == format!("{}.wasm", wapm.file_stem)
        }) {
            trace!(
                plugin = id,
                module = &fallback_module.name,
                "Found possible module with matching file name"
            );

            return self
                .download_plugin(id, &fallback_module.public_url, plugin_path)
                .await;
        }

        // Otherwise use the distribution download, which is typically an archive
        if let Some(download_url) = &package.distribution.download_url {
            trace!(
                plugin = id,
                "Using the distribution archive as a last resort"
            );

            return self.download_plugin(id, download_url, plugin_path).await;
        }

        Err(WarpgateError::WapmModuleMissing {
            package: wapm.package_name.to_owned(),
            version: version.to_owned(),
        }
        .into())
    }
}
