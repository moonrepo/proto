use crate::id::Id;
use crate::layout::Store;
use crate::registry::data::{PluginEntry, PluginRegistryDocument};
use crate::registry::registry_error::ProtoRegistryError;
use starbase_utils::{fs, json};
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::OnceCell;
use tracing::{debug, instrument};

pub struct ProtoRegistry {
    store: Store,
    community: OnceCell<Vec<PluginEntry>>,
    internal: OnceCell<Vec<PluginEntry>>,
    external: OnceCell<Vec<PluginEntry>>,
}

impl ProtoRegistry {
    pub(crate) fn new(store: Store) -> Self {
        debug!("Creating plugin registry");

        Self {
            store,
            community: OnceCell::new(),
            internal: OnceCell::new(),
            external: OnceCell::new(),
        }
    }

    #[instrument(skip(self))]
    pub async fn load_community_plugin(
        &self,
        id: &Id,
    ) -> Result<Option<&PluginEntry>, ProtoRegistryError> {
        Ok(self
            .load_community_plugins()
            .await?
            .into_iter()
            .find(|plugin| &plugin.id == id))
    }

    #[instrument(skip(self))]
    pub async fn load_community_plugins(&self) -> Result<Vec<&PluginEntry>, ProtoRegistryError> {
        let plugins = self
            .community
            .get_or_try_init(|| async {
                debug!("Loading community plugins registry data");

                self.load_plugins_from_registry(
                    self.store
                        .cache_dir
                        .join("registry/community-plugins.json"),
                    "https://raw.githubusercontent.com/moonrepo/proto/master/registry/data/community.json"
                        .into(),
                )
                .await
            })
            .await?;

        Ok(plugins.iter().collect())
    }

    pub async fn load_plugins(&self) -> Result<Vec<&PluginEntry>, ProtoRegistryError> {
        let mut plugins = self.load_internal_plugins().await?;
        plugins.extend(self.load_external_plugins().await?);

        Ok(plugins)
    }

    #[instrument(skip(self))]
    pub async fn load_internal_plugins(&self) -> Result<Vec<&PluginEntry>, ProtoRegistryError> {
        let plugins = self
            .internal
            .get_or_try_init(|| async {
                debug!("Loading built-in plugins registry data");

                self.load_plugins_from_registry(
                    self.store
                        .cache_dir
                        .join("registry/internal-plugins.json"),
                    "https://raw.githubusercontent.com/moonrepo/proto/master/registry/data/built-in.json"
                        .into(),
                )
                .await
            })
            .await?;

        Ok(plugins.iter().collect())
    }

    #[instrument(skip(self))]
    pub async fn load_external_plugins(&self) -> Result<Vec<&PluginEntry>, ProtoRegistryError> {
        let plugins = self
            .external
            .get_or_try_init(|| async {
                debug!("Loading third-party plugins registry data");

                self.load_plugins_from_registry(
                    self.store
                        .cache_dir
                        .join("registry/external-plugins.json"),
                    "https://raw.githubusercontent.com/moonrepo/proto/master/registry/data/third-party.json"
                        .into(),
                )
                .await
            })
            .await?;

        Ok(plugins.iter().collect())
    }

    async fn load_plugins_from_registry(
        &self,
        temp_file: PathBuf,
        data_url: String,
    ) -> Result<Vec<PluginEntry>, ProtoRegistryError> {
        // Cache should refresh every 24 hours
        let duration = Duration::from_secs(86400);

        if temp_file.exists() && !fs::is_stale(&temp_file, false, duration)? {
            debug!(file = ?temp_file, "Reading plugins data from local cache");

            let plugins: Vec<PluginEntry> = json::read_file(&temp_file)?;

            return Ok(plugins);
        }

        // Otherwise fetch from the upstream URL
        debug!(url = &data_url, "Loading plugins data from remote URL");

        let response = reqwest::get(&data_url)
            .await
            .map_err(|error| ProtoRegistryError::FailedRequest {
                url: data_url.clone(),
                error: Box::new(error),
            })?
            .error_for_status()
            .map_err(|error| ProtoRegistryError::FailedRequest {
                url: data_url,
                error: Box::new(error),
            })?;

        let data: PluginRegistryDocument =
            response
                .json()
                .await
                .map_err(|error| ProtoRegistryError::FailedParse {
                    error: Box::new(error),
                })?;

        // Cache the result for future requests
        json::write_file(temp_file, &data.plugins, false)?;

        Ok(data.plugins)
    }
}
