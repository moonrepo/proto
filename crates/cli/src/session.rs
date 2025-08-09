use crate::app::{App as CLI, Commands};
use crate::commands::clean::{CleanArgs, CleanTarget, internal_clean};
use crate::helpers::create_console_theme;
use crate::systems::*;
use crate::utils::progress_instance::ProgressInstance;
use crate::utils::tool_record::ToolRecord;
use async_trait::async_trait;
use proto_core::{
    ConfigMode, ProtoConfig, ProtoEnvironment, SCHEMA_PLUGIN_KEY, ToolContext, ToolSpec,
    load_schema_plugin_with_proto, load_tool, registry::ProtoRegistry,
};
use proto_core::{ProtoConfigError, ProtoLoaderError};
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
    pub inherit_local: bool,
    pub inherit_remote: bool,
    pub tools: FxHashSet<ToolContext>,
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
                | Commands::Setup(_)
                | Commands::Upgrade(_)
        )
    }

    pub fn create_registry(&self) -> ProtoRegistry {
        ProtoRegistry::new(Arc::clone(&self.env))
    }

    pub fn load_config(&self) -> Result<&ProtoConfig, ProtoConfigError> {
        self.env.load_config()
    }

    pub fn load_config_with_mode(
        &self,
        mode: ConfigMode,
    ) -> Result<&ProtoConfig, ProtoConfigError> {
        self.env.load_config_with_mode(mode)
    }

    pub async fn load_tool(&self, context: &ToolContext) -> Result<ToolRecord, ProtoLoaderError> {
        self.load_tool_with_options(context, LoadToolOptions::default())
            .await
    }

    #[tracing::instrument(name = "load_tool", skip(self))]
    pub async fn load_tool_with_options(
        &self,
        context: &ToolContext,
        options: LoadToolOptions,
    ) -> Result<ToolRecord, ProtoLoaderError> {
        let mut record = ToolRecord::new(load_tool(context, &self.env).await?);

        if options.inherit_remote {
            record.inherit_from_remote().await?;
        }

        if options.inherit_local {
            record.inherit_from_local(self.load_config()?);
        }

        if options.detect_version {
            record.detect_version_and_source().await;

            let spec = record
                .detected_version
                .clone()
                .unwrap_or_else(|| ToolSpec::parse("*").unwrap());

            record.tool.resolve_version(&spec, false).await?;
        }

        Ok(record)
    }

    /// Load tools that have a configured version.
    pub async fn load_tools(&self) -> Result<Vec<ToolRecord>, ProtoLoaderError> {
        self.load_tools_with_options(LoadToolOptions::default())
            .await
    }

    #[tracing::instrument(name = "load_tools", skip(self))]
    pub async fn load_tools_with_options(
        &self,
        mut options: LoadToolOptions,
    ) -> Result<Vec<ToolRecord>, ProtoLoaderError> {
        let config = self.env.load_config()?;

        // Gather the IDs of all possible tools. We can't just use the
        // `plugins` map, because some tools may not have a plugin entry,
        // for example, those using backends.
        let mut contexts = FxHashSet::default();
        contexts.extend(
            config
                .plugins
                .keys()
                .map(|id| ToolContext::new(id.to_owned())),
        );
        contexts.extend(config.versions.keys().cloned());

        // If no filter IDs provided, inherit the IDs from the current
        // config for every tool that has a version. Otherwise, we'll
        // load all tools, even built-ins, when the user isn't using them.
        // This causes quite a performance hit.
        if options.tools.is_empty() {
            options.tools.extend(config.versions.keys().cloned());
        }

        // Download the schema plugin before loading plugins.
        // We must do this here, otherwise when multiple schema
        // based tools are installed in parallel, they will
        // collide when attempting to download the schema plugin!
        load_schema_plugin_with_proto(&self.env).await?;

        let mut set = JoinSet::<Result<ToolRecord, ProtoLoaderError>>::new();
        let mut records = vec![];
        let opt_inherit_remote = options.inherit_remote;
        let opt_detect_version = options.detect_version;

        for context in contexts {
            if !options.tools.contains(&context) {
                continue;
            }

            // These shouldn't be treated as a "normal plugin"
            if context.id == SCHEMA_PLUGIN_KEY {
                continue;
            }

            let proto = Arc::clone(&self.env);

            set.spawn(async move {
                let mut record = ToolRecord::new(load_tool(&context, &proto).await?);

                if opt_inherit_remote {
                    record.inherit_from_remote().await?;
                }

                if opt_detect_version {
                    record.detect_version_and_source().await;
                }

                Ok(record)
            });
        }

        while let Some(result) = set.join_next().await {
            let mut record: ToolRecord = result.unwrap()?;

            if options.inherit_local {
                record.inherit_from_local(config);
            }

            records.push(record);
        }

        Ok(records)
    }

    /// Load all tools, even those not configured with a version.
    pub async fn load_all_tools(&self) -> Result<Vec<ToolRecord>, ProtoLoaderError> {
        self.load_all_tools_with_options(LoadToolOptions::default())
            .await
    }

    pub async fn load_all_tools_with_options(
        &self,
        mut options: LoadToolOptions,
    ) -> Result<Vec<ToolRecord>, ProtoLoaderError> {
        let config = self.load_config()?;

        let mut contexts = FxHashSet::default();
        contexts.extend(
            config
                .plugins
                .keys()
                .map(|id| ToolContext::new(id.to_owned())),
        );
        contexts.extend(config.versions.keys().cloned());

        options.tools = contexts;

        self.load_tools_with_options(options).await
    }

    pub async fn render_progress_loader(&self) -> ProgressInstance {
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

        ProgressInstance { reporter, handle }
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

        Ok(None)
    }

    async fn execute(&mut self) -> AppResult {
        remove_proto_shims(&self.env)?;
        clean_proto_backups(&self.env)?;

        if self.should_check_for_new_version() {
            check_for_new_version(&self.env, &self.console, &self.cli_version).await?;
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
