use crate::clients::*;
use crate::helpers::{
    create_cache_key, determine_cache_extension, download_from_url_to_file, move_or_unpack_download,
};
use crate::id::Id;
use crate::loader_error::WarpgateLoaderError;
use crate::protocols::{FileLoader, GitHubLoader, HttpLoader, LoadFrom, LoaderProtocol, OciLoader};
use crate::registry::RegistryConfig;
use once_cell::sync::OnceCell;
use starbase_styles::color;
use starbase_utils::fs;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::{instrument, trace, warn};
use warpgate_api::PluginLocator;

pub type OfflineChecker = Arc<fn() -> bool>;

/// A system for loading plugins from a locator strategy,
/// and caching the plugin file (`.wasm`) to the host's file system.
#[derive(Clone)]
pub struct PluginLoader {
    /// Duration in seconds in which to cache downloaded plugins.
    cache_duration: Duration,

    /// Loader for referencing local plugins using file paths.
    file_loader: OnceCell<FileLoader>,

    /// Loader for downloading plugins from GitHub releases.
    github_loader: OnceCell<GitHubLoader>,

    /// Instance of our HTTP client.
    http_client: OnceCell<Arc<HttpClient>>,

    /// Loader for acquiring plugins from URLs.
    http_loader: OnceCell<HttpLoader>,

    /// Options to pass to the HTTP client.
    http_options: HttpOptions,

    /// Checks whether there's an internet connection or not.
    offline_checker: Option<OfflineChecker>,

    /// Location where acquired plugins are stored.
    plugins_dir: PathBuf,

    /// Location where temporary files (like archives) are stored.
    temp_dir: PathBuf,

    /// Plugin registry locations
    registries: Vec<RegistryConfig>,

    /// A unique seed for generating hashes.
    seed: Option<String>,

    /// OCI client instance.
    oci_client: OnceCell<Arc<OciClient>>,

    /// Loader from downloading plugins from OCI registries.
    oci_loader: OnceCell<OciLoader>,
}

impl PluginLoader {
    /// Create a new loader that stores plugins and downloads in the provided directories.
    pub fn new<P: AsRef<Path>, T: AsRef<Path>>(plugins_dir: P, temp_dir: T) -> Self {
        let plugins_dir = plugins_dir.as_ref();

        trace!(cache_dir = ?plugins_dir, "Creating plugin loader");

        Self {
            cache_duration: Duration::from_secs(86400 * 30), // 30 days
            file_loader: OnceCell::new(),
            github_loader: OnceCell::new(),
            http_client: OnceCell::new(),
            http_loader: OnceCell::new(),
            http_options: HttpOptions::default(),
            oci_client: OnceCell::new(),
            oci_loader: OnceCell::new(),
            offline_checker: None,
            plugins_dir: plugins_dir.to_owned(),
            registries: vec![],
            seed: None,
            temp_dir: temp_dir.as_ref().to_owned(),
        }
    }

    /// Add an OCI registry as a backend.
    pub fn add_registry(&mut self, registry: RegistryConfig) {
        self.registries.push(registry);
    }

    /// Add multiple OCI registries as a backend.
    pub fn add_registries(&mut self, registries: Vec<RegistryConfig>) {
        for registry in registries {
            self.add_registry(registry);
        }
    }

    /// Return a file loader for use with [`FileLocator`]s.
    pub fn get_file_loader(&self) -> Result<&FileLoader, WarpgateLoaderError> {
        self.file_loader.get_or_try_init(|| Ok(FileLoader {}))
    }

    /// Return a GitHub loader for use with [`GitHubLocator`]s.
    pub fn get_github_loader(&self) -> Result<&GitHubLoader, WarpgateLoaderError> {
        self.github_loader.get_or_try_init(|| {
            Ok(GitHubLoader {
                client: Arc::clone(self.get_http_client()?),
            })
        })
    }

    /// Return an HTTP loader for use with [`UrlLocator`]s.
    pub fn get_http_loader(&self) -> Result<&HttpLoader, WarpgateLoaderError> {
        self.http_loader.get_or_try_init(|| Ok(HttpLoader {}))
    }

    /// Return an OCI loader for use with [`RegistryLocator`]s.
    pub fn get_oci_loader(&self) -> Result<&OciLoader, WarpgateLoaderError> {
        self.oci_loader.get_or_try_init(|| {
            Ok(OciLoader {
                client: Arc::clone(self.get_oci_client()?),
            })
        })
    }

    /// Return an OCI client, or create it if it does not exist.
    pub fn get_oci_client(&self) -> Result<&Arc<OciClient>, WarpgateHttpClientError> {
        self.oci_client
            .get_or_try_init(|| Ok(Arc::new(OciClient::default())))
    }

    /// Return the HTTP client, or create it if it does not exist.
    pub fn get_http_client(&self) -> Result<&Arc<HttpClient>, WarpgateHttpClientError> {
        self.http_client
            .get_or_try_init(|| create_http_client_with_options(&self.http_options).map(Arc::new))
    }

    /// Load a plugin using the provided locator. File system plugins are loaded directly,
    /// while remote/URL plugins are downloaded and cached.
    #[instrument(skip(self))]
    pub async fn load_plugin<I: AsRef<Id> + Debug, L: AsRef<PluginLocator> + Debug>(
        &self,
        id: I,
        locator: L,
    ) -> Result<PathBuf, WarpgateLoaderError> {
        let id = id.as_ref();
        let locator = locator.as_ref();

        trace!(
            id = id.as_str(),
            locator = locator.to_string(),
            "Loading plugin {}",
            color::id(id.as_str())
        );

        // Determine the source location
        let (source, is_latest) = match locator {
            PluginLocator::File(file) => {
                let loader = self.get_file_loader()?;

                (loader.load(id, file, &()).await?, loader.is_latest(file))
            }
            PluginLocator::GitHub(github) => {
                let loader = self.get_github_loader()?;

                (
                    loader.load(id, github, &()).await?,
                    loader.is_latest(github),
                )
            }
            PluginLocator::Url(url) => {
                let loader = self.get_http_loader()?;

                (loader.load(id, url, &()).await?, loader.is_latest(url))
            }
            PluginLocator::Registry(registry) => {
                let loader = self.get_oci_loader()?;

                (
                    loader.load(id, registry, &self.registries).await?,
                    loader.is_latest(registry),
                )
            }
        };

        // Check if destination already exists
        let cache_path = match &source {
            LoadFrom::Blob { ext, hash, .. } => self.create_cache_path(id, hash, ext, is_latest),
            LoadFrom::File(path) => {
                // Files ignore caching rules
                return Ok(path.to_path_buf());
            }
            LoadFrom::Url(url) => self.create_cache_path(
                id,
                create_cache_key(url, self.seed.as_deref()).as_str(),
                determine_cache_extension(url).unwrap_or(".wasm"),
                is_latest,
            ),
        };

        if self.is_cached(id, &cache_path)? {
            return Ok(cache_path);
        }

        // Acquire the source and write to the destination
        match source {
            LoadFrom::Blob { data, .. } => {
                fs::write_file(&cache_path, data)?;
            }
            LoadFrom::Url(url) => {
                self.download_plugin(id, &url, &cache_path).await?;
            }
            _ => {}
        };

        Ok(cache_path)
    }

    /// Create an absolute path to the plugin's destination file,
    /// located in the cached plugins directory.
    pub fn create_cache_path(&self, id: &Id, hash: &str, ext: &str, is_latest: bool) -> PathBuf {
        self.plugins_dir.join(format!(
            "{}{}{hash}{ext}",
            // Remove unwanted or unsafe file name characters
            id.as_str().replace(['/', '@', '.', ' '], ""),
            if is_latest { "-latest-" } else { "-" },
        ))
    }

    /// Check if the plugin has been acquired and is cached.
    /// If using a latest strategy (no explicit version or tag), the cache
    /// is only valid for a duration (to ensure not stale), otherwise forever.
    #[instrument(name = "is_plugin_cached", skip(self))]
    pub fn is_cached(&self, id: &Id, path: &Path) -> Result<bool, WarpgateLoaderError> {
        if !path.exists() {
            trace!(id = id.as_str(), "Plugin not cached, acquiring");

            return Ok(false);
        }

        if self.cache_duration.is_zero() {
            trace!(
                id = id.as_str(),
                "Plugin caching has been disabled, acquiring"
            );

            return Ok(false);
        }

        let mut cached =
            fs::is_stale(path, false, self.cache_duration, SystemTime::now())?.is_none();

        if !cached && self.is_offline() {
            cached = true;
        }

        if !cached && path.exists() {
            fs::remove_file(path)?;
        }

        if cached {
            trace!(id = id.as_str(), path = ?path, "Plugin already acquired and cached");
        } else {
            trace!(id = id.as_str(), path = ?path, "Plugin cached but stale, re-acquiring");
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

    /// Set the cache duration.
    pub fn set_cache_duration(&mut self, duration: Duration) {
        self.cache_duration = duration;
    }

    /// Set the options to pass to the HTTP client.
    pub fn set_http_client_options(&mut self, options: &HttpOptions) {
        options.clone_into(&mut self.http_options);
    }

    /// Set the function that checks for offline state.
    pub fn set_offline_checker(&mut self, op: fn() -> bool) {
        self.offline_checker = Some(Arc::new(op));
    }

    /// Set the provided value as a seed for generating hashes.
    pub fn set_seed(&mut self, value: &str) {
        self.seed = Some(value.to_owned());
    }

    #[instrument(skip(self))]
    async fn download_plugin(
        &self,
        id: &Id,
        source_url: &str,
        dest_file: &Path,
    ) -> Result<(), WarpgateLoaderError> {
        if self.is_offline() {
            return Err(WarpgateLoaderError::RequiredInternetConnection {
                message: "Unable to download plugin.".into(),
                url: source_url.to_owned(),
            });
        }

        trace!(
            id = id.as_str(),
            from = source_url,
            to = ?dest_file,
            "Downloading plugin from URL"
        );

        let temp_file = self.temp_dir.join(fs::file_name(dest_file));

        download_from_url_to_file(source_url, &temp_file, self.get_http_client()?).await?;
        move_or_unpack_download(&temp_file, dest_file)?;

        Ok(())
    }
}
