use crate::app::{App as CLI, Commands};
use crate::commands::clean::{internal_clean, CleanArgs};
use crate::systems::*;
use crate::utils::progress_instance::ProgressInstance;
use crate::utils::tool_record::ToolRecord;
use async_trait::async_trait;
use miette::IntoDiagnostic;
use proto_core::registry::ProtoRegistry;
use proto_core::{
    detect_version, load_schema_plugin_with_proto, load_tool_from_locator, load_tool_with_proto,
    ConfigMode, Id, ProtoConfig, ProtoEnvironment, Tool, UnresolvedVersionSpec, PROTO_PLUGIN_KEY,
    SCHEMA_PLUGIN_KEY,
};
use rustc_hash::FxHashSet;
use semver::Version;
use starbase::{AppResult, AppSession};
use starbase_console::ui::{style_to_color, ConsoleTheme, ProgressLoader, ProgressReporter};
use starbase_console::{Console, EmptyReporter};
use starbase_styles::Style;
use std::io::IsTerminal;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::debug;

#[derive(Debug, Default)]
pub struct LoadToolOptions {
    pub detect_version: bool,
    pub ids: FxHashSet<Id>,
    pub inherit_local: bool,
    pub inherit_remote: bool,
}

pub type ProtoConsole = Console<EmptyReporter>;

#[derive(Clone)]
pub struct ProtoSession {
    pub cli: CLI,
    pub cli_version: Version,
    pub console: ProtoConsole,
    pub env: Arc<ProtoEnvironment>,
}

impl ProtoSession {
    pub fn new(cli: CLI) -> Self {
        let mut console = Console::<EmptyReporter>::new(false);
        console.set_theme(ConsoleTheme::branded(style_to_color(Style::Shell)));
        console.set_reporter(EmptyReporter);

        Self {
            cli,
            cli_version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
            console,
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

    pub fn load_config(&self) -> miette::Result<&ProtoConfig> {
        self.env.load_config()
    }

    pub fn load_config_with_mode(&self, mode: ConfigMode) -> miette::Result<&ProtoConfig> {
        self.env.load_config_with_mode(mode)
    }

    pub async fn load_tool(&self, id: &Id) -> miette::Result<ToolRecord> {
        self.load_tool_with_options(id, LoadToolOptions::default())
            .await
    }

    #[tracing::instrument(name = "load_tool", skip(self))]
    pub async fn load_tool_with_options(
        &self,
        id: &Id,
        options: LoadToolOptions,
    ) -> miette::Result<ToolRecord> {
        let mut record = ToolRecord::new(load_tool_with_proto(id, &self.env).await?);

        if options.inherit_remote {
            record.inherit_from_remote().await?;
        }

        if options.inherit_local {
            record.inherit_from_local(self.load_config()?);
        }

        if options.detect_version {
            let version = detect_version(&record.tool, None)
                .await
                .unwrap_or_else(|_| UnresolvedVersionSpec::parse("*").unwrap());

            record.tool.resolve_version(&version, false).await?;
        }

        Ok(record)
    }

    pub async fn load_tools(&self) -> miette::Result<Vec<ToolRecord>> {
        self.load_tools_with_options(LoadToolOptions::default())
            .await
    }

    pub async fn load_tools_with_filters(
        &self,
        filters: FxHashSet<&Id>,
    ) -> miette::Result<Vec<ToolRecord>> {
        self.load_tools_with_options(LoadToolOptions {
            ids: filters.into_iter().cloned().collect(),
            ..Default::default()
        })
        .await
    }

    #[tracing::instrument(name = "load_tools", skip(self))]
    pub async fn load_tools_with_options(
        &self,
        options: LoadToolOptions,
    ) -> miette::Result<Vec<ToolRecord>> {
        let config = self.env.load_config()?;

        // Download the schema plugin before loading plugins.
        // We must do this here, otherwise when multiple schema
        // based tools are installed in parallel, they will
        // collide when attempting to download the schema plugin!
        load_schema_plugin_with_proto(&self.env).await?;

        let mut set = JoinSet::<miette::Result<ToolRecord>>::new();
        let mut records = vec![];
        let inherit_remote = options.inherit_remote;

        for (id, locator) in &config.plugins {
            if !options.ids.is_empty() && !options.ids.contains(id) {
                continue;
            }

            // These shouldn't be treated as a "normal plugin"
            if id == SCHEMA_PLUGIN_KEY || id == PROTO_PLUGIN_KEY {
                continue;
            }

            let id = id.to_owned();
            let locator = locator.to_owned();
            let proto = Arc::clone(&self.env);

            set.spawn(async move {
                let mut record = ToolRecord::new(load_tool_from_locator(id, proto, locator).await?);

                if inherit_remote {
                    record.inherit_from_remote().await?;
                }

                Ok(record)
            });
        }

        while let Some(result) = set.join_next().await {
            let mut record: ToolRecord = result.into_diagnostic()??;

            if options.inherit_local {
                record.inherit_from_local(config);
            }

            records.push(record);
        }

        Ok(records)
    }

    pub async fn load_proto_tool(&self) -> miette::Result<Tool> {
        load_tool_from_locator(
            Id::new(PROTO_PLUGIN_KEY)?,
            &self.env,
            self.env.load_config()?.builtin_proto_plugin(),
        )
        .await
    }

    pub fn render_progress_loader(&self) -> miette::Result<ProgressInstance> {
        use iocraft::prelude::element;

        let reporter = ProgressReporter::default();
        let reporter_clone = reporter.clone();
        let console = self.console.clone();

        let handle = tokio::task::spawn(async move {
            console
                .render_loop(element! {
                    ProgressLoader(reporter: reporter_clone)
                })
                .await
        });

        Ok(ProgressInstance { reporter, handle })
    }

    pub fn skip_prompts(&self, yes: bool) -> bool {
        yes || !std::io::stdout().is_terminal()
    }
}

#[async_trait]
impl AppSession for ProtoSession {
    async fn startup(&mut self) -> AppResult {
        self.env = Arc::new(detect_proto_env(&self.cli)?);

        Ok(None)
    }

    async fn analyze(&mut self) -> AppResult {
        load_proto_configs(&self.env)?;
        download_versioned_proto_tool(self).await?;

        Ok(None)
    }

    async fn execute(&mut self) -> AppResult {
        clean_proto_backups(&self.env)?;

        if self.should_check_for_new_version() {
            check_for_new_version(Arc::clone(&self.env), &self.cli_version).await?;
        }

        Ok(None)
    }

    async fn shutdown(&mut self) -> AppResult {
        if self.should_check_for_new_version() && self.env.load_config()?.settings.auto_clean {
            debug!("Auto-clean enabled, starting clean");

            internal_clean(self, &CleanArgs::default(), true).await?;
        }

        self.console.out.flush()?;
        self.console.err.flush()?;

        Ok(None)
    }
}
