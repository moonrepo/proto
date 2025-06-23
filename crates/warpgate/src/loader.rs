use crate::client::{HttpClient, HttpOptions, create_http_client_with_options};
use crate::client_error::WarpgateClientError;
use crate::helpers::{
    create_cache_key, determine_cache_extension, download_from_url_to_file, move_or_unpack_download,
};
use crate::id::Id;
use crate::loader_error::WarpgateLoaderError;
use crate::{OciRegistry, endpoints::*};
use docker_credential::{CredentialRetrievalError, DockerCredential};
use oci_client::Reference;
use oci_client::client::{Client, ClientConfig};
use oci_client::manifest;
use oci_client::secrets::RegistryAuth;
use once_cell::sync::OnceCell;
use starbase_archive::is_supported_archive_extension;
use starbase_styles::color;
use starbase_utils::fs;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::{debug, instrument, trace, warn};
use warpgate_api::{FileLocator, GitHubLocator, PluginLocator, UrlLocator};
pub type OfflineChecker = Arc<fn() -> bool>;

/// A system for loading plugins from a locator strategy,
/// and caching the `.wasm` file to the host's file system.
#[derive(Clone)]
pub struct PluginLoader {
    /// Duration in seconds in which to cache downloaded plugins.
    cache_duration: Duration,

    /// Instance of our HTTP client.
    http_client: OnceCell<Arc<HttpClient>>,

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
            cache_duration: Duration::from_secs(86400 * 30), // 30 days
            http_client: OnceCell::new(),
            http_options: HttpOptions::default(),
            offline_checker: None,
            plugins_dir: plugins_dir.to_owned(),
            temp_dir: temp_dir.as_ref().to_owned(),
            seed: None,
        }
    }

    /// Return the HTTP client, or create it if it does not exist.
    pub fn get_client(&self) -> Result<&Arc<HttpClient>, WarpgateClientError> {
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
        registries: Vec<OciRegistry>,
    ) -> Result<PathBuf, WarpgateLoaderError> {
        let id = id.as_ref();
        let locator = locator.as_ref();

        trace!(
            id = id.as_str(),
            locator = locator.to_string(),
            "Loading plugin {}",
            color::id(id.as_str())
        );

        match locator {
            PluginLocator::File(file) => self.load_plugin_from_file(id, file).await,
            PluginLocator::GitHub(github) => self.load_plugin_from_github(id, github).await,
            PluginLocator::Url(url) => self.load_plugin_from_url(id, url).await,
            PluginLocator::Oci(oci) => self.load_plugin_from_oci(id, oci, registries).await,
        }
    }

    /// Load a plugin from the file system.
    #[instrument(skip(self))]
    pub async fn load_plugin_from_file<I: AsRef<Id> + Debug>(
        &self,
        id: I,
        locator: &FileLocator,
    ) -> Result<PathBuf, WarpgateLoaderError> {
        let id = id.as_ref();
        let path = locator.get_resolved_path();

        if path.exists() {
            trace!(
                id = id.as_str(),
                path = ?path,
                "Using source file",
            );

            Ok(path)
        } else {
            Err(WarpgateLoaderError::MissingSourceFile {
                id: id.to_owned(),
                path: path.to_path_buf(),
            })
        }
    }

    /// Load a plugin from a GitHub release.
    #[instrument(skip(self))]
    pub async fn load_plugin_from_github<I: AsRef<Id> + Debug>(
        &self,
        id: I,
        locator: &GitHubLocator,
    ) -> Result<PathBuf, WarpgateLoaderError> {
        let id = id.as_ref();

        self.download_plugin_from_github(id, locator).await
    }

    /// Load a plugin from a secure URL.
    #[instrument(skip(self))]
    pub async fn load_plugin_from_url<I: AsRef<Id> + Debug>(
        &self,
        id: I,
        locator: &UrlLocator,
    ) -> Result<PathBuf, WarpgateLoaderError> {
        let id = id.as_ref();
        let url = &locator.url;

        self.download_plugin(
            id,
            url,
            self.create_cache_path(id, url, url.contains("latest")),
        )
        .await
    }

    /// Load a plugin from an OCI registry.
    #[instrument(skip(self))]
    pub async fn load_plugin_from_oci<I: AsRef<Id> + Debug>(
        &self,
        id: I,
        locator: &warpgate_api::OciLocator,
        registries: Vec<OciRegistry>,
    ) -> Result<PathBuf, WarpgateLoaderError> {
        let id = id.as_ref();
        let client = Client::new(ClientConfig::default());

        let location = if locator.image.as_str().starts_with("oci://") {
            &locator.image.as_str()[6..]
        } else {
            locator.image.as_str()
        };

        if let Ok(reference) = Reference::try_from(location.to_string()) {
            let registries: Option<Vec<OciRegistry>> =
                if let Some(reg) = reference.resolve_registry().strip_suffix('/') {
                    Some(vec![OciRegistry {
                        registry: reg.to_string(),
                        organization: None,
                    }])
                } else {
                    // If no registry is specified, use the configured registries
                    Some(registries)
                };

            let tag: String = if let Some(tag) = reference.tag() {
                tag.into()
            } else {
                "latest".to_string()
            };

            for registry in registries.unwrap() {
                debug!("Searching registry {} for {id}:{tag}", registry.registry);

                let auth = match docker_credential::get_credential(&registry.registry) {
                    Err(CredentialRetrievalError::ConfigNotFound) => RegistryAuth::Anonymous,
                    Err(CredentialRetrievalError::NoCredentialConfigured) => {
                        RegistryAuth::Anonymous
                    }
                    Err(e) => panic!("Error handling docker configuration file: {}", e),
                    Ok(DockerCredential::UsernamePassword(username, password)) => {
                        debug!("Found docker credentials");
                        RegistryAuth::Basic(username, password)
                    }
                    Ok(DockerCredential::IdentityToken(_)) => {
                        debug!(
                            "Cannot use contents of docker config, identity token not supported. Using anonymous auth"
                        );
                        RegistryAuth::Anonymous
                    }
                };

                let image_ref = if let Some(org) = &registry.organization {
                    // If the registry has an organization, prepend it to the image reference
                    format!("{}/{}/{}:{tag}", registry.registry, org, id)
                } else {
                    format!("{}/{}:{tag}", registry.registry, id)
                };

                let reference = Reference::try_from(image_ref.as_str()).unwrap();
                let image = client
                    .pull(&reference, &auth, vec![manifest::WASM_LAYER_MEDIA_TYPE])
                    .await
                    .map_err(|e| WarpgateLoaderError::OCIReferenceError {
                        message: e.to_string(),
                        location: location.to_string(),
                    })?;

                if let Some(layer) = image.layers.first() {
                    let file_path = self.plugins_dir.join(format!("{id}.wasm"));
                    std::fs::write(&file_path, &layer.data).map_err(|e| {
                        WarpgateLoaderError::OCIReferenceError {
                            message: e.to_string(),
                            location: location.to_string(),
                        }
                    })?;

                    return Ok(file_path);
                }
            }
            Err(WarpgateLoaderError::OCIReferenceError {
                message: "No valid registry found or no valid layer.".into(),
                location: location.to_string(),
            })
        } else {
            Err(WarpgateLoaderError::OCIReferenceError {
                message: "No valid registry found or no valid layer.".into(),
                location: location.to_string(),
            })
        }

        // let reference = Reference::try_from(location)
        //     .map_err(|e| WarpgateLoaderError::OCIReferenceError { message: e.to_string(), location: location.to_string() })?;

        // if let Some(registry) = reference.resolve_registry().strip_prefix("https://") {
        //     warn!("Using registry: {}", registry);
        // } else {
        //     warn!("Using default registry: {}", reference.resolve_registry());
        // }

        // let server = reference
        //     .resolve_registry()
        //     .strip_suffix('/')
        //     .unwrap_or_else(|| reference.resolve_registry());

        // let auth = match docker_credential::get_credential(server) {
        //     Err(CredentialRetrievalError::ConfigNotFound) => RegistryAuth::Anonymous,
        //     Err(CredentialRetrievalError::NoCredentialConfigured) => RegistryAuth::Anonymous,
        //     Err(e) => panic!("Error handling docker configuration file: {}", e),
        //     Ok(DockerCredential::UsernamePassword(username, password)) => {
        //         debug!("Found docker credentials");
        //         RegistryAuth::Basic(username, password)
        //     }
        //     Ok(DockerCredential::IdentityToken(_)) => {
        //         warn!("Cannot use contents of docker config, identity token not supported. Using anonymous auth");
        //         RegistryAuth::Anonymous
        //     }
        // };

        // let image = client.pull(&reference, &auth, vec![manifest::WASM_LAYER_MEDIA_TYPE]).await
        //     .map_err(|e| WarpgateLoaderError::OCIReferenceError { message: e.to_string(), location: location.to_string() })?;

        // if let Some(layer) = image.layers.first() {
        //     let file_path = self.plugins_dir.join(format!("{id}.wasm"));
        //     std::fs::write(&file_path, &layer.data)
        //         .map_err(|e| WarpgateLoaderError::OCIReferenceError { message: e.to_string(), location: location.to_string() })?;

        //     Ok(file_path)
        // } else {
        //     Err(WarpgateLoaderError::OCIReferenceError { message: "Layer is not valid.".into(), location: location.to_string() })
        // }
    }

    /// Create an absolute path to the plugin's destination file, located in the plugins directory.
    /// Hash the source URL to ensure uniqueness of each plugin + version combination.
    pub fn create_cache_path(&self, id: &Id, url: &str, is_latest: bool) -> PathBuf {
        self.plugins_dir.join(format!(
            "{}{}{}{}",
            // Remove unwanted or unsafe file name characters
            id.as_str().replace(['/', '@', '.', ' '], ""),
            if is_latest { "-latest-" } else { "-" },
            create_cache_key(url, self.seed.as_deref()),
            determine_cache_extension(url).unwrap_or(".wasm"),
        ))
    }

    /// Check if the plugin has been downloaded and is cached.
    /// If using a latest strategy (no explicit version or tag), the cache
    /// is only valid for a duration (to ensure not stale), otherwise forever.
    #[instrument(name = "is_plugin_cached", skip(self))]
    pub fn is_cached(&self, id: &Id, path: &Path) -> Result<bool, WarpgateLoaderError> {
        if !path.exists() {
            trace!(id = id.as_str(), "Plugin not cached, downloading");

            return Ok(false);
        }

        if self.cache_duration.is_zero() {
            trace!(
                id = id.as_str(),
                "Plugin caching has been disabled, downloading"
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

    /// Set the cache duration.
    pub fn set_cache_duration(&mut self, duration: Duration) {
        self.cache_duration = duration;
    }

    /// Set the options to pass to the HTTP client.
    pub fn set_client_options(&mut self, options: &HttpOptions) {
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
        dest_file: PathBuf,
    ) -> Result<PathBuf, WarpgateLoaderError> {
        if self.is_cached(id, &dest_file)? {
            return Ok(dest_file);
        }

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

        let temp_file = self.temp_dir.join(fs::file_name(&dest_file));

        download_from_url_to_file(source_url, &temp_file, self.get_client()?).await?;
        move_or_unpack_download(&temp_file, &dest_file)?;

        Ok(dest_file)
    }

    #[instrument(skip(self))]
    async fn download_plugin_from_github(
        &self,
        id: &Id,
        github: &GitHubLocator,
    ) -> Result<PathBuf, WarpgateLoaderError> {
        // Check the cache first using the repository slug as the seed,
        // so that we can avoid making unnecessary HTTP requests.
        let plugin_path = self.create_cache_path(
            id,
            github.repo_slug.as_str(),
            github.tag.is_none() && github.project_name.is_none(),
        );

        if self.is_cached(id, &plugin_path)? {
            return Ok(plugin_path);
        }

        if self.is_offline() {
            return Err(WarpgateLoaderError::RequiredInternetConnection {
                message: format!(
                    "Unable to download plugin {} from GitHub.",
                    PluginLocator::GitHub(Box::new(github.to_owned()))
                ),
                url: "https://api.github.com".into(),
            });
        }

        // Fetch all tags then find a matching tag + release
        let tags_url = format!("https://api.github.com/repos/{}/tags", github.repo_slug);
        let found_tag;

        trace!(
            id = id.as_str(),
            tag = github.tag.as_ref(),
            tag_prefix = github.project_name.as_ref(),
            tags_url = &tags_url,
            "Attempting to find a matching tag",
        );

        if let Some(tag) = &github.tag {
            found_tag = Some(tag.to_owned())
        } else if let Some(tag_prefix) = &github.project_name {
            found_tag = send_github_request::<Vec<GitHubApiTag>>(self.get_client()?, &tags_url)
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
                repo_slug: github.repo_slug.to_owned(),
            });
        };

        let release_url = if release_tag == "latest" {
            format!(
                "https://api.github.com/repos/{}/releases/latest",
                github.repo_slug,
            )
        } else {
            format!(
                "https://api.github.com/repos/{}/releases/tags/{release_tag}",
                github.repo_slug,
            )
        };

        trace!(
            id = id.as_str(),
            release_url = &release_url,
            release_tag = &release_tag,
            "Attempting to download plugin from GitHub release",
        );

        let release: GitHubApiRelease =
            send_github_request(self.get_client()?, &release_url).await?;

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
        if let Some(tag_prefix) = &github.project_name {
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

                    return self
                        .download_plugin(id, &asset.browser_download_url, plugin_path)
                        .await;
                }
            }
        }

        Err(WarpgateLoaderError::MissingGitHubAsset {
            id: id.to_owned(),
            repo_slug: github.repo_slug.to_owned(),
            tag: release_tag,
        })
    }
}
