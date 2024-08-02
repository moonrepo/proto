use crate::app::{App as CLI, Commands};
use crate::commands::clean::{internal_clean, CleanArgs};
use crate::systems::*;
use async_trait::async_trait;
use miette::IntoDiagnostic;
use proto_core::registry::ProtoRegistry;
use proto_core::{
    load_schema_plugin_with_proto, load_tool_from_locator, load_tool_with_proto, Id,
    ProtoEnvironment, Tool, SCHEMA_PLUGIN_KEY,
};
use rustc_hash::FxHashSet;
use starbase::{AppResult, AppSession};
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::debug;

#[derive(Clone)]
pub struct ProtoSession {
    pub cli: CLI,
    pub cli_version: String,
    pub env: Arc<ProtoEnvironment>,
}

impl ProtoSession {
    pub fn new(cli: CLI) -> Self {
        Self {
            cli,
            cli_version: env!("CARGO_PKG_VERSION").to_owned(),
            env: Arc::new(ProtoEnvironment::default()),
        }
    }

    pub fn should_check_for_new_version(&self) -> bool {
        !matches!(
            self.cli.command,
            Commands::Activate(_)
                | Commands::Bin(_)
                | Commands::Clean(_)
                | Commands::Completions(_)
                | Commands::Run(_)
                | Commands::Setup(_)
                | Commands::Upgrade(_)
        )
    }

    pub fn create_registry(&self) -> ProtoRegistry {
        ProtoRegistry::new(Arc::clone(&self.env))
    }

    pub async fn load_tool(&self, id: &Id) -> miette::Result<Tool> {
        load_tool_with_proto(id, &self.env).await
    }

    pub async fn load_tools(&self) -> miette::Result<Vec<Tool>> {
        self.load_tools_with_filters(FxHashSet::default()).await
    }

    #[tracing::instrument(name = "load_tools", skip_all)]
    pub async fn load_tools_with_filters(
        &self,
        filter: FxHashSet<&Id>,
    ) -> miette::Result<Vec<Tool>> {
        let config = self.env.load_config()?;

        // Download the schema plugin before loading plugins.
        // We must do this here, otherwise when multiple schema
        // based tools are installed in parallel, they will
        // collide when attempting to download the schema plugin!
        load_schema_plugin_with_proto(&self.env).await?;

        let mut set = JoinSet::new();
        let mut tools = vec![];

        for (id, locator) in &config.plugins {
            if !filter.is_empty() && !filter.contains(id) {
                continue;
            }

            // This shouldn't be treated as a "normal plugin"
            if id == SCHEMA_PLUGIN_KEY {
                continue;
            }

            let id = id.to_owned();
            let locator = locator.to_owned();
            let proto = Arc::clone(&self.env);

            set.spawn(async move { load_tool_from_locator(id, proto, locator).await });
        }

        while let Some(result) = set.join_next().await {
            tools.push(result.into_diagnostic()??);
        }

        Ok(tools)
    }
}

#[async_trait]
impl AppSession for ProtoSession {
    async fn startup(&mut self) -> AppResult {
        self.env = Arc::new(detect_proto_env()?);

        sync_current_proto_tool(&self.env, &self.cli_version)?;

        Ok(())
    }

    async fn analyze(&mut self) -> AppResult {
        load_proto_configs(&self.env)?;
        download_versioned_proto_tool(&self.env).await?;

        Ok(())
    }

    async fn execute(&mut self) -> AppResult {
        if self.should_check_for_new_version() {
            check_for_new_version(Arc::clone(&self.env)).await?;

            if self.env.load_config()?.settings.auto_clean {
                debug!("Auto-clean enabled, starting clean");

                internal_clean(self, CleanArgs::default(), true).await?;
            }
        }

        Ok(())
    }
}
