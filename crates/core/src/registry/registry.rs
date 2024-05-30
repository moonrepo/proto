use crate::proto::ProtoEnvironment;
use crate::registry::data::{PluginEntry, PluginRegistryDocument};
use crate::registry::registry_error::ProtoRegistryError;
use starbase_utils::{fs, json};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

pub struct ProtoRegistry {
    env: Arc<ProtoEnvironment>,
}

impl ProtoRegistry {
    pub fn new(env: Arc<ProtoEnvironment>) -> Self {
        Self { env }
    }

    pub async fn load_external_plugins(&self) -> miette::Result<Vec<PluginEntry>> {
        let temp_file = self.env.store.temp_dir.join("registry-plugins.json");

        // Cache should refresh every 24 hours
        let now = SystemTime::now();
        let duration = Duration::from_secs(86400);

        if temp_file.exists() {
            if fs::is_stale(&temp_file, false, duration, now)?.is_none() {
                let data: PluginRegistryDocument = json::read_file(&temp_file)?;

                return Ok(data.plugins);
            }
        }

        // Otherwise fetch from the upstream URL
        let url = "https://raw.githubusercontent.com/moonrepo/proto/develop-0.36/registry/data/third-party.json";

        let data: PluginRegistryDocument = reqwest::get(url)
            .await
            .map_err(|error| ProtoRegistryError::RequestFailed {
                url: url.to_owned(),
                error: Box::new(error),
            })?
            .json()
            .await
            .map_err(|error| ProtoRegistryError::ParseFailed {
                error: Box::new(error),
            })?;

        // Cache the result for future requests
        json::write_file(temp_file, &data, false)?;

        Ok(data.plugins)
    }
}
