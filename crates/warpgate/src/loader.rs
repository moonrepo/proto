use crate::clients::*;
use crate::helpers::{
    determine_cache_extension, download_from_url_to_file, extract_file_name_from_url, hash_sha256,
    move_or_unpack_download,
};
use crate::loader_error::WarpgateLoaderError;
use crate::protocols::{
    DataLoader, FileLoader, GitHubLoader, HttpLoader, LoadFrom, LoaderProtocol, OciLoader,
};
use crate::registry::RegistryConfig;
use once_cell::sync::OnceCell;
use starbase_styles::color;
use starbase_utils::fs::FsError;
use starbase_utils::net::DownloadOptions;
use starbase_utils::{fs, path};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{instrument, trace, warn};
use warpgate_api::{Id, PluginLocator};

pub type OfflineChecker = Arc<fn() -> bool>;

/// A system for loading plugins from a locator strategy,
/// and caching the plugin file (`.wasm`) to the host's file system.
pub struct PluginLoader {
    /// Duration in seconds in which to cache downloaded plugins.
    cache_duration: Duration,

    /// Loader for referencing local plugins using byte streams.
    data_loader: OnceCell<DataLoader>,

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

    /// In-process locks for plugins currently being downloaded,
    /// to prevent multiple simultaneous downloads of the same plugin.
    locks: scc::HashMap<String, Arc<Mutex<()>>>,

    /// Checks whether there's an internet connection or not.
    offline_checker: Option<OfflineChecker>,

    /// Location where acquired plugins are stored.
    plugins_dir: PathBuf,

    /// Location where temporary files (like archives) are stored.
    temp_dir: PathBuf,

    /// Plugin registry locations
    registries: Vec<RegistryConfig>,

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
            data_loader: OnceCell::new(),
            file_loader: OnceCell::new(),
            github_loader: OnceCell::new(),
            http_client: OnceCell::new(),
            http_loader: OnceCell::new(),
            http_options: HttpOptions::default(),
            locks: scc::HashMap::new(),
            oci_client: OnceCell::new(),
            oci_loader: OnceCell::new(),
            offline_checker: None,
            plugins_dir: plugins_dir.to_owned(),
            registries: vec![],
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

    /// Return a data loader for use with [`DataLocator`]s.
    pub fn get_data_loader(&self) -> Result<&DataLoader, WarpgateLoaderError> {
        self.data_loader.get_or_try_init(|| Ok(DataLoader {}))
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

    /// Return the HTTP client, or create it if it does not exist.
    pub fn get_http_client(&self) -> Result<&Arc<HttpClient>, WarpgateHttpClientError> {
        self.http_client
            .get_or_try_init(|| create_http_client_with_options(&self.http_options).map(Arc::new))
    }

    /// Return an OCI client, or create it if it does not exist.
    pub fn get_oci_client(&self) -> Result<&Arc<OciClient>, WarpgateHttpClientError> {
        self.oci_client
            .get_or_try_init(|| Ok(Arc::new(OciClient::default())))
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
            PluginLocator::Data(data) => {
                let loader = self.get_data_loader()?;

                (loader.load(id, data, &()).await?, loader.is_latest(data))
            }
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

        // Create a lock before checking the cache, so that subsequent requests for
        // the same plugin will wait for the first one to finish downloading, and will
        // then hit the cache instead of downloading again
        let cache_path = match source {
            LoadFrom::Blob {
                data, ext, hash, ..
            } => {
                let entry = self.locks.entry_async(hash.clone()).await.or_default();
                let _lock = entry.lock().await;

                let cache_path = self.create_cache_path(id, &hash, &ext, is_latest);

                if !self.is_cached(id, &cache_path)? {
                    fs::write_file_with_lock(&cache_path, data)?;
                }

                cache_path
            }
            LoadFrom::File(path) => path.to_path_buf(),
            LoadFrom::Url(url) => {
                let entry = self.locks.entry_async(url.clone()).await.or_default();
                let _lock = entry.lock().await;

                let cache_path = self.create_cache_path(
                    id,
                    hash_sha256(&url).as_str(),
                    determine_cache_extension(&url).unwrap_or(".wasm"),
                    is_latest,
                );

                if !self.is_cached(id, &cache_path)? {
                    self.download_plugin(id, &url, &cache_path).await?;
                }

                cache_path
            }
        };

        Ok(cache_path)
    }

    /// Create an absolute path to the plugin's destination file,
    /// located in the cached plugins directory.
    pub fn create_cache_path(&self, id: &Id, hash: &str, ext: &str, is_latest: bool) -> PathBuf {
        self.plugins_dir.join(format!(
            "{}-{}{}.{}",
            path::encode_component(id.as_str()),
            if is_latest { "latest-" } else { "" },
            hash,
            ext.trim_start_matches('.')
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

        if self.cache_duration.is_zero() && !self.is_offline() {
            trace!(
                id = id.as_str(),
                "Plugin caching has been disabled, acquiring"
            );

            return Ok(false);
        }

        let mut cached = !fs::is_stale(path, false, self.cache_duration)?;

        if !cached && self.is_offline() {
            cached = true;
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

        // Ensure different URLs with the same file name don't conflict in
        // the temp directory by hashing the URL into the path
        let temp_path = self
            .temp_dir
            .join(hash_sha256(source_url))
            .join(extract_file_name_from_url(source_url));

        // Do not truncate the file as another process may be writing to it,
        // instead create if missing and then acquire an exclusive lock
        let file = fs::create_file_if_missing(&temp_path)?;

        let cleanup = || {
            // Don't fail and avoid unlocking!
            let _ = fs::remove_file(&temp_path);

            fs::release_lock(&temp_path, &file)?;

            Ok::<_, WarpgateLoaderError>(())
        };

        // Acquire lock before downloading and hold until after archive extraction,
        // and the temp file is moved/copied to the destination
        fs::acquire_exclusive_lock(&temp_path, &file)?;

        if dest_file.exists() {
            trace!(
                id = id.as_str(),
                path = ?dest_file,
                "Plugin downloaded by another process while waiting for lock, skipping download",
            );

            cleanup()?;

            return Ok(());
        }

        if let Err(error) = download_from_url_to_file(
            source_url,
            &temp_path,
            DownloadOptions {
                downloader: Some(Box::new(self.get_http_client()?.create_downloader())),
                file: Some(file.try_clone().map_err(|error| FsError::Create {
                    path: temp_path.to_path_buf(),
                    error: Box::new(error),
                })?),
                // Don't lock within this function as we locked it above!
                lock: false,
                ..Default::default()
            },
        )
        .await
        {
            // Don't swallow the download error
            let _ = cleanup();

            return Err(error);
        }

        if let Err(error) = move_or_unpack_download(&temp_path, dest_file) {
            // Don't swallow the move/unpack error
            let _ = cleanup();

            return Err(error);
        }

        cleanup()?;
        drop(file);

        Ok(())
    }
}
