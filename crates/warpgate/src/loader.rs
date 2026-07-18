use crate::clients::*;
use crate::helpers::{download_from_url_to_file, extract_file_name_from_url, move_or_unpack_file};
use crate::loader_error::WarpgateLoaderError;
use crate::protocols::{
    DataLoader, FileLoader, GitHubLoader, HttpLoader, LoadFrom, LoaderProtocol, OciLoader,
};
use crate::registry::RegistryConfig;
use once_cell::sync::OnceCell;
use starbase_styles::color;
use starbase_utils::fs::{self, FileLock};
use starbase_utils::net::DownloadOptions;
use starbase_utils::{hash, path};
use std::fmt::{Debug, Display};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{instrument, trace};
use warpgate_api::{Id, PluginLocator};

pub type OfflineChecker = Arc<fn() -> bool>;

#[derive(Debug)]
pub struct LoadedPlugin {
    pub cached: bool,
    pub path: PathBuf,
}

/// A system for loading plugins from a locator strategy,
/// and caching the plugin file (`.wasm`) to the host's file system.
pub struct PluginLoader {
    /// Duration in seconds in which to cache downloaded plugins.
    cache_duration: Duration,

    /// Loader for referencing local plugins using byte streams.
    data_loader: OnceCell<DataLoader>,

    /// List of supported plugin file extensions, used for caching and validation.
    extensions: Vec<String>,

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
            extensions: vec!["wasm".into()],
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

    /// Add multiple supported plugin file extensions.
    pub fn add_extensions(&mut self, extensions: Vec<String>) {
        self.extensions.extend(extensions);
    }

    /// Add multiple OCI registries as a backend.
    pub fn add_registries(&mut self, registries: Vec<RegistryConfig>) {
        self.registries.extend(registries);
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
                registries: self.registries.clone(),
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
            .get_or_try_init(|| create_oci_client_with_options(&self.http_options).map(Arc::new))
    }

    /// Load a plugin using the provided locator. File system plugins are loaded directly,
    /// while remote/URL plugins are downloaded and cached.
    #[instrument(skip(self))]
    pub async fn load_plugin<I: AsRef<Id> + Debug, L: AsRef<PluginLocator> + Debug>(
        &self,
        id: I,
        locator: L,
    ) -> Result<PathBuf, WarpgateLoaderError> {
        Ok(self.load_plugin_with_metadata(id, locator).await?.path)
    }

    /// Load a plugin using the provided locator and return cache metadata.
    #[instrument(skip(self))]
    pub async fn load_plugin_with_metadata<
        I: AsRef<Id> + Debug,
        L: AsRef<PluginLocator> + Debug,
    >(
        &self,
        id: I,
        locator: L,
    ) -> Result<LoadedPlugin, WarpgateLoaderError> {
        let id = id.as_ref();
        let locator = locator.as_ref();

        trace!(
            id = id.as_str(),
            locator = locator.to_string(),
            "Loading plugin {}",
            color::id(id.as_str())
        );

        match locator {
            PluginLocator::Data(data) => {
                self.check_cache_or_save(
                    id,
                    &**data,
                    hash::sha256::from_bytes(data.bytes.as_deref().unwrap_or(data.data.as_bytes())),
                    || self.get_data_loader(),
                )
                .await
            }
            PluginLocator::File(file) => {
                let loader = self.get_file_loader()?;

                // File paths are used as-is and completely ignore the locking and caching system,
                // as it's assumed the user is managing these themselves
                match loader.load(id, file).await? {
                    LoadFrom::File(path) => Ok(LoadedPlugin {
                        cached: false,
                        path,
                    }),
                    _ => unreachable!(),
                }
            }
            PluginLocator::GitHub(github) => {
                self.check_cache_or_save(
                    id,
                    &**github,
                    hash::sha256::from_bytes(locator.to_string()),
                    || self.get_github_loader(),
                )
                .await
            }
            PluginLocator::Url(url) => {
                self.check_cache_or_save(id, &**url, hash::sha256::from_bytes(&url.url), || {
                    self.get_http_loader()
                })
                .await
            }
            PluginLocator::Registry(registry) => {
                self.check_cache_or_save(
                    id,
                    &**registry,
                    hash::sha256::from_bytes(locator.to_string()),
                    || self.get_oci_loader(),
                )
                .await
            }
        }
    }

    /// Create an absolute path to the plugin's destination file,
    /// located in the cached plugins directory.
    pub fn create_cache_path(&self, id: &Id, hash: &str, ext: &str, is_latest: bool) -> PathBuf {
        self.plugins_dir.join(format!(
            "{}-{}{}.{ext}",
            path::encode_component(id.as_str()),
            if is_latest { "latest-" } else { "" },
            hash,
        ))
    }

    /// Create an absolute path to a temporary file for the plugin,
    /// which will be used for locking and downloading to.
    pub fn create_temp_path(&self, id: &Id, hash: &str, is_lock: bool) -> PathBuf {
        self.temp_dir.join(format!(
            "{}-{hash}{}",
            path::encode_component(id.as_str()),
            if is_lock { ".lock" } else { "" },
        ))
    }

    /// Check if the plugin has been acquired and is cached.
    /// If using a latest strategy (no explicit version or tag), the cache
    /// is only valid for a duration (to ensure not stale), otherwise forever.
    pub fn is_cached(&self, id: &Id, path: &Path) -> Result<bool, WarpgateLoaderError> {
        if !path.exists() {
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

            // Remove it so that subsequent checks don't cache hit
            let _ = fs::remove_file(path);
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

    async fn check_cache_or_save<'a, T: Display, L, F>(
        &self,
        id: &'a Id,
        locator: &'a T,
        hash: String,
        get_loader: F,
    ) -> Result<LoadedPlugin, WarpgateLoaderError>
    where
        L: LoaderProtocol<T> + 'a,
        F: FnOnce() -> Result<&'a L, WarpgateLoaderError>,
    {
        // Create a lock before checking the cache, so that subsequent requests for
        // the same plugin will wait for the first one to finish loading, and will
        // then hit the cache instead of loading again
        let entry = self.locks.entry_async(hash.clone()).await.or_default();
        let _lock = entry.lock().await;

        // Find a cache file with the provided hash. We need to check all possible
        // file extensions, because certain loaders and archives may produce a different
        // extension than "wasm" (e.g. "toml" or "yaml")
        trace!(id = id.as_str(), "Checking if plugin has been cached");

        let loader = get_loader()?;
        let is_latest = loader.is_latest(locator);

        for ext in &self.extensions {
            let cache_path = self.create_cache_path(id, &hash, ext, is_latest);

            if self.is_cached(id, &cache_path)? {
                let lock_path = self.create_temp_path(id, &hash, true);

                // Although the cache file exists, another process may be in the middle of
                // downloading and saving it, so we need to acquire an exclusive lock on the
                // lock file to ensure it's fully written before we return the path
                if lock_path.exists()
                    && let Ok(lock_file) = fs::open_file(&lock_path)
                {
                    fs::acquire_exclusive_lock(lock_path, &lock_file)?;
                }

                return Ok(LoadedPlugin {
                    cached: true,
                    path: cache_path,
                });
            }
        }

        trace!(id = id.as_str(), "Plugin not cached, acquiring");

        if loader.requires_online() && self.is_offline() {
            return Err(WarpgateLoaderError::RequiredInternetConnection {
                message: "Unable to download plugin.".into(),
                locator: locator.to_string(),
            });
        }

        let cache_path = self
            .save_to_cache(id, hash, is_latest, loader.load(id, locator).await?)
            .await?;

        Ok(LoadedPlugin {
            cached: false,
            path: cache_path,
        })
    }

    fn determine_cache_extension(&self, value: &str) -> Option<&str> {
        self.extensions
            .iter()
            .find(|ext| value.ends_with(&format!(".{ext}")))
            .map(|ext| ext.as_str())
    }

    #[instrument(skip(self, source))]
    async fn save_to_cache(
        &self,
        id: &Id,
        hash: String,
        is_latest: bool,
        source: LoadFrom<'_>,
    ) -> Result<PathBuf, WarpgateLoaderError> {
        let mut dest_file = self.create_cache_path(id, &hash, "wasm", is_latest);
        let mut temp_file = self.create_temp_path(id, &hash, false);
        let mut is_archive = false;

        if let Some(ext) = source.is_archive() {
            temp_file.set_extension(ext);
            is_archive = true;
        }

        // Do not truncate the file as another process may be writing to it,
        // instead create if missing and then acquire an exclusive lock.
        // Hold until after archive extraction and the temp file is moved/copied
        // to the destination.
        let mut lock = FileLock::new_async(self.create_temp_path(id, &hash, true)).await?;

        // Remove the temp file when unlocking or dropping the reference,
        // which happens on success or failure!
        lock.remove_on_unlock();

        // Save the plugin to a temporary file first, then move it to the destination, to ensure
        // atomicity and prevent partially written files in case of failure or multiple processes
        // trying to acquire the same plugin at the same time.
        match source {
            LoadFrom::Blob { data, ext, .. } => {
                trace!(
                    id = id.as_str(),
                    to = ?temp_file,
                    "Saving plugin from bytes"
                );

                // We know the final file extension ahead of time as it is
                // explicitly provided by the loader (from an OCI layer)
                dest_file.set_extension(&ext);

                fs::write_file(&temp_file, data)?;
            }
            LoadFrom::Url(url) => {
                // Attempt to extract the final file extension from the URL,
                // so that we can update the destination similar to the blob case
                let file_name = extract_file_name_from_url(&url);

                if let Some(ext) = self.determine_cache_extension(&file_name) {
                    dest_file.set_extension(ext);
                }

                if !is_archive && dest_file.exists() {
                    trace!(
                        id = id.as_str(),
                        path = ?dest_file,
                        "Plugin downloaded by another process while waiting for lock, skipping download",
                    );

                    return Ok(dest_file);
                }

                trace!(
                    id = id.as_str(),
                    from = ?url,
                    to = ?temp_file,
                    "Downloading plugin from URL"
                );

                // Now download the file to the temporary location
                download_from_url_to_file(
                    &url,
                    &temp_file,
                    DownloadOptions {
                        downloader: Some(Box::new(self.get_http_client()?.create_downloader())),
                        ..Default::default()
                    },
                )
                .await?;
            }
            LoadFrom::File(_) => {
                unimplemented!();
            }
        };

        // If the file is an archive, unpack it, otherwise move it to the destination!
        move_or_unpack_file(&temp_file, &mut dest_file, &self.extensions)?;

        // Remove the temp file if it was not moved
        if temp_file.exists() {
            let _ = fs::remove_file(temp_file);
        }

        Ok(dest_file)
    }
}
