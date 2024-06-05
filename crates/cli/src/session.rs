use crate::app::{App as CLI, Commands};
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
use tokio::task;

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
            cli_version: String::new(),
            env: Arc::new(ProtoEnvironment::default()),
        }
    }

    pub fn should_check_for_new_version(&self) -> bool {
        !matches!(
            self.cli.command,
            Commands::Bin(_)
                | Commands::Completions(_)
                | Commands::Run(_)
                | Commands::Setup(_)
                | Commands::Upgrade
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

        let mut futures = vec![];
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

            futures.push(tokio::spawn(async move {
                load_tool_from_locator(id, proto, locator).await
            }));
        }

        for future in futures {
            tools.push(future.await.into_diagnostic()??);
        }

        Ok(tools)
    }
}

#[async_trait]
impl AppSession for ProtoSession {
    async fn startup(&mut self) -> AppResult {
        self.cli_version = setup_env_vars(self.cli.log.as_ref());
        self.env = Arc::new(detect_proto_env()?);

        Ok(())
    }

    async fn analyze(&mut self) -> AppResult {
        load_proto_configs(&self.env)?;

        Ok(())
    }

    async fn execute(&mut self) -> AppResult {
        if self.should_check_for_new_version() {
            task::spawn(check_for_new_version(Arc::clone(&self.env)))
                .await
                .into_diagnostic()??;
        }

        Ok(())
    }
}
