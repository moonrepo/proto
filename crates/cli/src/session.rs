use crate::app::{App as CLI, Commands};
use crate::commands::clean::{CleanArgs, CleanTarget, internal_clean};
use crate::helpers::create_console_theme;
use crate::systems::*;
use crate::utils::progress_instance::ProgressInstance;
use crate::utils::tool_record::ToolRecord;
use async_trait::async_trait;
use miette::IntoDiagnostic;
use proto_core::registry::ProtoRegistry;
use proto_core::{
    Backend, ConfigMode, Id, PROTO_PLUGIN_KEY, ProtoConfig, ProtoEnvironment, SCHEMA_PLUGIN_KEY,
    Tool, ToolSpec, UnresolvedVersionSpec, load_schema_plugin_with_proto, load_tool,
    load_tool_from_locator,
};
use rustc_hash::FxHashSet;
use semver::Version;
use starbase::{AppResult, AppSession};
use starbase_console::ui::{OwnedOrShared, Progress, ProgressDisplay, ProgressReporter};
use starbase_console::{Console, EmptyReporter};
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
        let env = ProtoEnvironment::default();

        let mut console = Console::<EmptyReporter>::new(false);
        console.set_theme(create_console_theme());
        console.set_reporter(EmptyReporter);

        Self {
            cli,
            cli_version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
            console,
            env: Arc::new(env),
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

    pub async fn load_tool(&self, id: &Id, backend: Option<Backend>) -> miette::Result<ToolRecord> {
        self.load_tool_with_options(id, backend, LoadToolOptions::default())
            .await
    }

    #[tracing::instrument(name = "load_tool", skip(self))]
    pub async fn load_tool_with_options(
        &self,
        id: &Id,
        backend: Option<Backend>,
        options: LoadToolOptions,
    ) -> miette::Result<ToolRecord> {
        let mut record = ToolRecord::new(load_tool(id, &self.env, backend).await?);

        if options.inherit_remote {
            record.inherit_from_remote().await?;
        }

        if options.inherit_local {
            record.inherit_from_local(self.load_config()?);
        }

        if options.detect_version {
            record.detect_version().await;

            let spec = ToolSpec::new(
                record
                    .detected_version
                    .clone()
                    .unwrap_or_else(|| UnresolvedVersionSpec::parse("*").unwrap()),
            );

            record.tool.resolve_version_with_spec(&spec, false).await?;
        }

        Ok(record)
    }

    pub async fn load_tools(&self) -> miette::Result<Vec<ToolRecord>> {
        self.load_tools_with_options(LoadToolOptions::default())
            .await
    }

    #[tracing::instrument(name = "load_tools", skip(self))]
    pub async fn load_tools_with_options(
        &self,
        mut options: LoadToolOptions,
    ) -> miette::Result<Vec<ToolRecord>> {
        let config = self.env.load_config()?;

        // Gather the IDs of all possible tools. We can't just use the
        // `plugins` map, because some tools may not have a plugin entry,
        // for example, those using backends.
        let mut ids = FxHashSet::default();
        ids.extend(config.plugins.keys());
        ids.extend(config.versions.keys());

        // If no filter IDs provided, inherit the IDs from the current
        // config for every tool that has a version. Otherwise, we'll
        // load all tools, even built-ins, when the user isn't using them.
        // This causes quite a performance hit.
        if options.ids.is_empty() {
            options.ids.extend(config.versions.keys().cloned());
        }

        // Download the schema plugin before loading plugins.
        // We must do this here, otherwise when multiple schema
        // based tools are installed in parallel, they will
        // collide when attempting to download the schema plugin!
        load_schema_plugin_with_proto(&self.env).await?;

        let mut set = JoinSet::<miette::Result<ToolRecord>>::new();
        let mut records = vec![];
        let opt_inherit_remote = options.inherit_remote;
        let opt_detect_version = options.detect_version;

        for id in ids {
            if !options.ids.is_empty() && !options.ids.contains(id) {
                continue;
            }

            // These shouldn't be treated as a "normal plugin"
            if id == SCHEMA_PLUGIN_KEY || id == PROTO_PLUGIN_KEY {
                continue;
            }

            let id = id.to_owned();
            let proto = Arc::clone(&self.env);

            set.spawn(async move {
                let mut record = ToolRecord::new(load_tool(&id, &proto, None).await?);

                if opt_inherit_remote {
                    record.inherit_from_remote().await?;
                }

                if opt_detect_version {
                    record.detect_version().await;
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

    #[allow(dead_code)]
    pub async fn load_all_tools(&self) -> miette::Result<Vec<ToolRecord>> {
        self.load_all_tools_with_options(LoadToolOptions::default())
            .await
    }

    pub async fn load_all_tools_with_options(
        &self,
        mut options: LoadToolOptions,
    ) -> miette::Result<Vec<ToolRecord>> {
        let config = self.load_config()?;

        let mut set = FxHashSet::default();
        set.extend(config.versions.keys().collect::<Vec<_>>());
        set.extend(config.plugins.keys().collect::<Vec<_>>());

        options.ids = set.into_iter().cloned().collect();

        self.load_tools_with_options(options).await
    }

    pub async fn load_proto_tool(&self) -> miette::Result<Tool> {
        load_tool_from_locator(
            Id::new(PROTO_PLUGIN_KEY)?,
            &self.env,
            self.env.load_config()?.builtin_proto_plugin(),
        )
        .await
    }

    pub async fn render_progress_loader(&self) -> miette::Result<ProgressInstance> {
        use iocraft::prelude::element;

        let reporter = Arc::new(ProgressReporter::default());
        let reporter_clone = OwnedOrShared::Shared(reporter.clone());
        let console = self.console.clone();

        let handle = tokio::task::spawn(async move {
            console
                .render_interactive(element! {
                    Progress(
                        display: ProgressDisplay::Loader,
                        reporter: reporter_clone,
                    )
                })
                .await
        });

        // Wait a bit for the component to be rendered
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        Ok(ProgressInstance { reporter, handle })
    }

    pub fn should_print_json(&self) -> bool {
        self.cli.json
    }

    pub fn should_skip_prompts(&self) -> bool {
        self.cli.yes || std::env::var("CI").is_ok_and(|v| !v.is_empty())
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
        if matches!(
            self.cli.command,
            Commands::Activate(_)
                | Commands::Install(_)
                | Commands::Outdated(_)
                | Commands::Regen(_)
                | Commands::Status(_)
        ) && self.env.load_config()?.settings.auto_clean
        {
            debug!("Auto-clean enabled, starting clean");

            // Skip prompts!
            self.cli.yes = true;

            internal_clean(
                self,
                &CleanArgs {
                    target: CleanTarget::All,
                    days: 30, // Doesn't inherit clap defaults
                },
            )
            .await?;
        }

        self.console.out.flush()?;
        self.console.err.flush()?;

        Ok(None)
    }
}
