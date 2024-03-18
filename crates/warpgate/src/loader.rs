use crate::client::{create_http_client_with_options, HttpOptions};
use crate::endpoints::*;
use crate::error::WarpgateError;
use crate::helpers::{
    determine_cache_extension, download_from_url_to_file, move_or_unpack_download,
};
use crate::id::Id;
use once_cell::sync::OnceCell;
use sha2::{Digest, Sha256};
use starbase_archive::is_supported_archive_extension;
use starbase_styles::color;
use starbase_utils::fs;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::trace;
use warpgate_api::{GitHubLocator, PluginLocator};

pub type OfflineChecker = Arc<fn() -> bool>;

/// A system for loading plugins from a locator strategy,
/// and caching the `.wasm` file to the host's file system.
#[derive(Clone)]
pub struct PluginLoader {
    /// Instance of our HTTP client.
    http_client: OnceCell<reqwest::Client>,

    /// Options to pass to the HTTP client.
    http_options: HttpOptions,

    /// Checks whether there's an internet connection or not.
    offline_checker: Option<OfflineChecker>,

    /// Location where downloaded `.wasm` plugins are stored.
    plugins_dir: PathBuf,

    /// Location where temporary files (like archives) are stored.
    temp_dir: PathBuf,

    /// A unique seed for generating hashes.
    seed: Option<String>,
}

impl PluginLoader {
    /// Create a new loader that stores plugins and downloads in the provided directories.
    pub fn new<P: AsRef<Path>, T: AsRef<Path>>(plugins_dir: P, temp_dir: T) -> Self {
        let plugins_dir = plugins_dir.as_ref();

        trace!(cache_dir = ?plugins_dir, "Creating plugin loader");

        Self {
            http_client: OnceCell::new(),
            http_options: HttpOptions::default(),
            offline_checker: None,
            plugins_dir: plugins_dir.to_owned(),
            temp_dir: temp_dir.as_ref().to_owned(),
            seed: None,
        }
    }

    /// Return the HTTP client, or create it if it does not exist.
    pub fn get_client(&self) -> miette::Result<&reqwest::Client> {
        self.http_client
            .get_or_try_init(|| create_http_client_with_options(&self.http_options))
    }

    /// Load a plugin using the provided locator. File system plugins are loaded directly,
    /// while remote/URL plugins are downloaded and cached.
    pub async fn load_plugin<I: AsRef<Id>, L: AsRef<PluginLocator>>(
        &self,
        id: I,
        locator: L,
    ) -> miette::Result<PathBuf> {
        let id = id.as_ref();
        let locator = locator.as_ref();

        trace!(
            id = id.as_str(),
            "Loading plugin {}",
            color::id(id.as_str())
        );

        match locator {
            PluginLocator::SourceFile { path, .. } => {
                let path = path
                    .canonicalize()
                    .map_err(|_| WarpgateError::SourceFileMissing {
                        id: id.to_owned(),
                        path: path.to_path_buf(),
                    })?;

                if path.exists() {
                    trace!(
                        id = id.as_str(),
                        path = ?path,
                        "Using source file",
                    );

                    Ok(path)
                } else {
                    Err(WarpgateError::SourceFileMissing {
                        id: id.to_owned(),
                        path: path.to_path_buf(),
                    }
                    .into())
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
        }
    }

    /// Create an absolute path to the plugin's destination file, located in the plugins directory.
    /// Hash the source URL to ensure uniqueness of each plugin + version combination.
    pub fn create_cache_path(&self, id: &Id, url: &str, is_latest: bool) -> PathBuf {
        let mut sha = Sha256::new();
        sha.update(url);

        if let Some(seed) = &self.seed {
            sha.update(seed);
        }

        // Remove unwanted or unsafe file name characters
        let safe_id = id.as_str().replace(['/', '@', '.', ' '], "");

        self.plugins_dir.join(format!(
            "{}{}{:x}{}",
            safe_id,
            if is_latest { "-latest-" } else { "-" },
            sha.finalize(),
            determine_cache_extension(url)
        ))
    }

    /// Check if the plugin has been downloaded and is cached.
    /// If using a latest strategy (no explicit version or tag), the cache
    /// is only valid for 7 days (to ensure not stale), otherwise forever.
    pub fn is_cached(&self, id: &Id, path: &Path) -> miette::Result<bool> {
        if !path.exists() {
            trace!(id = id.as_str(), "Plugin not cached, downloading");

            return Ok(false);
        }

        let metadata = fs::metadata(path)?;

        let mut cached = if let Ok(filetime) = metadata.created().or_else(|_| metadata.modified()) {
            let days = if fs::file_name(path).contains("-latest-") {
                7
            } else {
                30
            };

            filetime > SystemTime::now() - Duration::from_secs(86400 * days)
        } else {
            false
        };

        if !cached && self.is_offline() {
            cached = true;
        }

        if !cached {
            fs::remove_file(path)?;
        }

        if cached {
            trace!(id = id.as_str(), path = ?path, "Plugin already downloaded and cached");
        } else {
            trace!(id = id.as_str(), path = ?path, "Plugin cached but stale, re-downloading");
        }

        Ok(cached)
    }

    /// Check for an internet connection.
    pub fn is_offline(&self) -> bool {
        self.offline_checker
            .as_ref()
            .map(|op| op())
            .unwrap_or_default()
    }

    /// Set the options to pass to the HTTP client.
    pub fn set_client_options(&mut self, options: &HttpOptions) {
        self.http_options = options.to_owned();
    }

    /// Set the function that checks for offline state.
    pub fn set_offline_checker(&mut self, op: fn() -> bool) {
        self.offline_checker = Some(Arc::new(op));
    }

    /// Set the provided value as a seed for generating hashes.
    pub fn set_seed(&mut self, value: &str) {
        self.seed = Some(value.to_owned());
    }

    async fn download_plugin(
        &self,
        id: &Id,
        source_url: &str,
        dest_file: PathBuf,
    ) -> miette::Result<PathBuf> {
        if self.is_cached(id, &dest_file)? {
            return Ok(dest_file);
        }

        if self.is_offline() {
            return Err(WarpgateError::InternetConnectionRequired {
                message: "Unable to download plugin.".into(),
                url: source_url.to_owned(),
            }
            .into());
        }

        trace!(
            id = id.as_str(),
            from = source_url,
            to = ?dest_file,
            "Downloading plugin from URL"
        );

        let temp_file = self.temp_dir.join(fs::file_name(&dest_file));

        download_from_url_to_file(source_url, &temp_file, self.get_client()?).await?;
        move_or_unpack_download(&temp_file, &dest_file)?;

        Ok(dest_file)
    }

    async fn download_plugin_from_github(
        &self,
        id: &Id,
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
            id = id.as_str(),
            api_url = &api_url,
            release_tag = &release_tag,
            "Attempting to download plugin from GitHub release",
        );

        let handle_error = |error: reqwest::Error| WarpgateError::Http {
            error,
            url: api_url.clone(),
        };

        if self.is_offline() {
            return Err(WarpgateError::InternetConnectionRequired {
                message: format!(
                    "Unable to download plugin {} from GitHub.",
                    PluginLocator::GitHub(github.to_owned())
                ),
                url: api_url,
            }
            .into());
        }

        // Otherwise make an HTTP request to the GitHub releases API,
        // and loop through the assets to find a matching one.
        let mut request = self.get_client()?.get(&api_url);

        if let Ok(auth_token) = env::var("GITHUB_TOKEN") {
            request = request.bearer_auth(auth_token);
        }

        let response = request.send().await.map_err(handle_error)?;
        let release: GitHubApiRelease = response.json().await.map_err(handle_error)?;

        // Find a direct WASM asset first
        for asset in &release.assets {
            if asset.content_type == "application/wasm" || asset.name.ends_with(".wasm") {
                trace!(
                    id = id.as_str(),
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
            if asset.name == github.file_prefix
                || (asset.name.starts_with(&github.file_prefix)
                    && is_supported_archive_extension(&PathBuf::from(&asset.name)))
            {
                trace!(
                    id = id.as_str(),
                    asset = &asset.name,
                    "Found possible asset as an archive"
                );

                return self
                    .download_plugin(id, &asset.browser_download_url, plugin_path)
                    .await;
            }
        }

        Err(WarpgateError::GitHubAssetMissing {
            id: id.to_owned(),
            repo_slug: github.repo_slug.to_owned(),
            tag: release_tag,
        }
        .into())
    }
}
