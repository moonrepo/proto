use crate::app::{App as CLI, Commands, StdoutOwner};
use crate::commands::clean::{CleanArgs, CleanTarget, internal_clean};
use crate::helpers::create_console_theme;
use crate::systems::*;
use crate::utils::progress_instance::{ProgressInstance, monitor_non_tty_progress};
use crate::utils::tool_record::ToolRecord;
use async_trait::async_trait;
use proto_core::flow::resolve::Resolver;
use proto_core::{
    ConfigMode, ProtoConfig, ProtoEnvironment, SCHEMA_PLUGIN_KEY, ToolContext, ToolSpec, Version,
    load_schema_plugin_with_proto, load_tool,
    registry::ProtoRegistry,
    reporter::{ProtoConsole, ProtoReporter, ReporterFormat},
};
use proto_core::{ProtoConfigError, ProtoLoaderError, UnresolvedVersionSpec};
use rustc_hash::FxHashSet;
use starbase::{AppResult, AppSession};
use starbase_console::Console;
use starbase_console::ui::{OwnedOrShared, Progress, ProgressDisplay, ProgressReporter};
use starbase_utils::envx;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{debug, instrument};

pub type SessionResult = AppResult<miette::Report>;

#[derive(Debug, Default)]
pub struct LoadToolOptions {
    pub all: bool,
    pub contexts: FxHashSet<ToolContext>,
    pub detect_version: bool,
    pub inherit_local: bool,
    pub inherit_remote: Option<UnresolvedVersionSpec>,
}

#[derive(Clone)]
pub struct ProtoSession {
    pub cli: CLI,
    pub cli_version: Version,
    pub console: ProtoConsole,
    pub env: Arc<ProtoEnvironment>,
}

fn should_check_for_new_version(cli: &CLI) -> bool {
    if cli.stdout_owner() != StdoutOwner::Reporter {
        return false;
    }

    !matches!(
        &cli.command,
        Commands::Activate(_)
            | Commands::Bin(_)
            | Commands::Clean(_)
            | Commands::Exec(_)
            | Commands::Run(_)
            | Commands::Setup(_)
            | Commands::Upgrade(_)
    )
}

impl ProtoSession {
    pub fn new(cli: CLI) -> Self {
        let mut env = ProtoEnvironment::default();
        env.otel_enabled = cli.otel;

        let mut console = Console::<ProtoReporter>::new(false);
        console.set_theme(create_console_theme());
        console.set_reporter(if env.test_only {
            ProtoReporter::new_testing()
        } else if cli.stdout_owner() == StdoutOwner::Reporter {
            ProtoReporter::new(cli.reporter_format())
        } else {
            ProtoReporter::new_stderr(cli.reporter_format())
        });

        Self {
            cli,
            cli_version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
            console,
            env: Arc::new(env),
        }
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

    #[instrument(name = "load_tool", skip(self))]
    pub async fn load_tool_with_options(
        &self,
        context: &ToolContext,
        options: LoadToolOptions,
    ) -> Result<ToolRecord, ProtoLoaderError> {
        let mut record = ToolRecord::new(load_tool(context, &self.env).await?);

        if let Some(spec) = &options.inherit_remote {
            record.inherit_from_remote(spec).await?;
        }

        if options.inherit_local {
            record.inherit_from_local(self.load_config()?);
        }

        if options.detect_version {
            record.detect_version_and_source().await;

            let mut spec = record
                .detected_version
                .clone()
                .unwrap_or_else(|| ToolSpec::parse("*").unwrap());

            Resolver::resolve(&record.tool, &mut spec, false).await?;

            record.spec = spec;
        }

        Ok(record)
    }

    pub async fn load_tool_dependencies(
        &self,
        parent: ToolRecord,
    ) -> Result<Vec<ToolRecord>, ProtoLoaderError> {
        let mut seen = FxHashSet::from_iter([parent.context.clone()]);
        let mut queue = VecDeque::from(parent.metadata.requires.clone());
        let mut tools = vec![parent];

        while let Some(id) = queue.pop_front() {
            let context = ToolContext::parse(&id)?;

            if !seen.insert(context.clone()) {
                continue;
            }

            let Ok(mut tool) = self
                .load_tool_with_options(
                    &context,
                    LoadToolOptions {
                        detect_version: true,
                        ..Default::default()
                    },
                )
                .await
            else {
                continue;
            };

            let Some(mut spec) = tool.detected_version.clone() else {
                debug!(
                    tool = context.as_str(),
                    "Could not detect a version for the required tool, not adding to the environment",
                );

                continue;
            };

            if Resolver::resolve(&tool, &mut spec, true).await.is_err() || !tool.is_installed(&spec)
            {
                debug!(
                    tool = context.as_str(),
                    "The required tool is not installed, not adding to the environment",
                );

                continue;
            }

            debug!(
                tool = context.as_str(),
                version = spec.get_resolved_version().to_string(),
                "Adding the required tool to the environment",
            );

            tool.detected_version = Some(spec);
            queue.extend(tool.metadata.requires.iter().cloned());
            tools.push(tool);
        }

        Ok(tools)
    }

    /// Load tools that have a configured version.
    pub async fn load_tools(&self) -> Result<Vec<ToolRecord>, ProtoLoaderError> {
        self.load_tools_with_options(LoadToolOptions::default())
            .await
    }

    #[instrument(name = "load_tools", skip(self))]
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
                .tools
                .keys()
                .map(|id| ToolContext::new(id.to_owned())),
        );
        contexts.extend(config.versions.keys().cloned());

        // If no filter IDs provided, inherit the IDs from the current
        // config for every tool that has a version. Otherwise, we'll
        // load all tools, even built-ins, when the user isn't using them.
        // This causes quite a performance hit.
        if options.contexts.is_empty() {
            if options.all {
                options.contexts.extend(contexts.clone());
            } else {
                options.contexts.extend(config.versions.keys().cloned());
            }
        }

        // Download the schema plugin before loading plugins.
        // We must do this here, otherwise when multiple schema
        // based tools are installed in parallel, they will
        // collide when attempting to download the schema plugin!
        if !contexts.is_empty() {
            load_schema_plugin_with_proto(&self.env).await?;
        }

        let mut set = JoinSet::<Result<ToolRecord, ProtoLoaderError>>::new();
        let mut records = vec![];

        for context in contexts {
            if !options.contexts.contains(&context) {
                continue;
            }

            // These shouldn't be treated as a "normal plugin"
            if context.id == SCHEMA_PLUGIN_KEY {
                continue;
            }

            let proto = Arc::clone(&self.env);
            let opt_inherit_remote = options.inherit_remote.clone();
            let opt_detect_version = options.detect_version;

            set.spawn(Box::pin(async move {
                let mut record = ToolRecord::new(load_tool(&context, &proto).await?);

                if let Some(spec) = &opt_inherit_remote {
                    record.inherit_from_remote(spec).await?;
                }

                if opt_detect_version {
                    record.detect_version_and_source().await;
                }

                Ok(record)
            }));
        }

        while let Some(result) = set.join_next().await {
            let mut record: ToolRecord =
                result.map_err(|error| ProtoLoaderError::FailedJoin {
                    error: Box::new(error),
                })??;

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
        options.all = true;

        self.load_tools_with_options(options).await
    }

    pub async fn render_progress_loader(&self) -> ProgressInstance {
        use iocraft::prelude::element;

        let reporter = ProgressReporter::default();
        let console = self.console.clone();

        let handle = if self.is_tty() {
            let reporter_clone = OwnedOrShared::Owned(reporter.clone());

            tokio::spawn(Box::pin(async move {
                console
                    .render_interactive(element! {
                        Progress(
                            display: ProgressDisplay::Loader,
                            reporter: reporter_clone,
                        )
                    })
                    .await
            }))
        } else {
            monitor_non_tty_progress(console, reporter.clone(), None)
        };

        // Wait a bit for the component to be rendered
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        ProgressInstance { reporter, handle }
    }

    pub fn is_json_format(&self) -> bool {
        self.console.is_json_format()
    }

    pub fn is_tty(&self) -> bool {
        !envx::bool_var("NO_TTY") && self.console.out.is_terminal()
    }

    pub fn should_skip_prompts(&self) -> bool {
        self.cli.yes || ci_env::is_ci() || cd_env::is_cd()
    }
}

#[async_trait]
impl AppSession for ProtoSession {
    type Error = miette::Report;

    async fn startup(&mut self) -> AppResult<Self::Error> {
        if ai_env::is_ai_agent()
            && self.cli.stdout_owner() == StdoutOwner::Reporter
            && self.cli.reporter_format() == ReporterFormat::Ndjson
        {
            self.console.message("Detected an AI agent environment, printing as NDJSON. Trace logs are written to stderr, while user-facing logs are written to stdout.")?;
        }

        self.env = Arc::new(detect_proto_env(&self.cli)?);

        Ok(None)
    }

    async fn analyze(&mut self) -> AppResult<Self::Error> {
        load_proto_configs(&self.env)?;

        Ok(None)
    }

    async fn execute(&mut self) -> AppResult<Self::Error> {
        remove_proto_shims(&self.env)?;
        clean_proto_backups(&self.env)?;

        if should_check_for_new_version(&self.cli) {
            check_for_new_version(&self.env, &self.console, &self.cli_version).await?;
        }

        Ok(None)
    }

    async fn shutdown(&mut self) -> AppResult<Self::Error> {
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

        self.console.flush_json()?;
        self.console.out.flush()?;
        self.console.err.flush()?;

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn version_checks_require_reporter_owned_stdout() {
        let shell = CLI::try_parse_from(["proto", "shell", "--shell", "bash"]).unwrap();
        assert!(should_check_for_new_version(&shell));

        let status = CLI::try_parse_from(["proto", "status"]).unwrap();
        assert!(should_check_for_new_version(&status));

        let mcp = CLI::try_parse_from(["proto", "mcp"]).unwrap();
        assert!(!should_check_for_new_version(&mcp));

        let mcp_info = CLI::try_parse_from(["proto", "mcp", "--info"]).unwrap();
        assert!(should_check_for_new_version(&mcp_info));
    }
}
