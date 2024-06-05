use crate::app::{App as CLI, Commands};
use crate::systems::*;
use async_trait::async_trait;
use miette::IntoDiagnostic;
use proto_core::ProtoEnvironment;
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
                .into_diagnostic()?;
        }

        Ok(())
    }
}
