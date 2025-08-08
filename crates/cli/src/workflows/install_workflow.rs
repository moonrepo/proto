use crate::commands::pin::internal_pin;
use crate::components::{InstallAllProgress, InstallProgress, InstallProgressProps};
use crate::error::ProtoCliError;
use crate::session::ProtoConsole;
use crate::shell::{self, Export};
use crate::telemetry::*;
use crate::utils::tool_record::ToolRecord;
use iocraft::element;
use proto_core::flow::install::{InstallOptions, InstallPhase};
use proto_core::utils::log::LogWriter;
use proto_core::{Id, LockRecord, PinLocation, ToolSpec};
use proto_pdk_api::{
    HookFunction, InstallHook, InstallStrategy, PluginFunction, Switch, SyncShellProfileInput,
    SyncShellProfileOutput,
};
use starbase_console::ConsoleError;
use starbase_console::ui::{OwnedOrShared, ProgressDisplay, ProgressReporter, ProgressState};
use starbase_console::utils::formats::format_duration;
use starbase_shell::ShellType;
use starbase_styles::{apply_style_tags, color};
use starbase_utils::env::bool_var;
use std::collections::BTreeMap;
use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::{debug, warn};

#[derive(Debug)]
pub enum InstallOutcome {
    AlreadyInstalled(Id),
    Installed(Id),
    FailedToInstall(Id),
}

pub struct InstallWorkflowParams {
    pub force: bool,
    pub log_writer: Option<LogWriter>,
    pub multiple: bool,
    pub passthrough_args: Vec<String>,
    pub pin_to: Option<PinLocation>,
    pub quiet: bool,
    pub skip_prompts: bool,
    pub strategy: Option<InstallStrategy>,
}

pub struct InstallWorkflow {
    pub console: ProtoConsole,
    pub progress_reporter: ProgressReporter,
    pub tool: ToolRecord,
}

impl InstallWorkflow {
    pub fn new(tool: ToolRecord, console: ProtoConsole) -> Self {
        if tool.metadata.unstable.is_enabled() {
            warn!(
                "{} is currently unstable. {}",
                tool.get_name(),
                match &tool.metadata.unstable {
                    Switch::Message(msg) => {
                        msg
                    }
                    _ => {
                        ""
                    }
                }
            );
        }

        Self {
            console,
            progress_reporter: ProgressReporter::default(),
            tool,
        }
    }

    pub fn is_build(&self, strategy: Option<InstallStrategy>) -> bool {
        matches!(
            strategy.unwrap_or(self.tool.metadata.default_install_strategy),
            InstallStrategy::BuildFromSource
        )
    }

    pub async fn install(
        &mut self,
        spec: ToolSpec,
        params: InstallWorkflowParams,
    ) -> Result<InstallOutcome, ProtoCliError> {
        let started = Instant::now();

        self.progress_reporter.set_message(format!(
            "Installing {} with specification <versionalt>{}</versionalt>",
            self.tool.get_name(),
            spec
        ));

        // Disable version caching and always use the latest when installing
        self.tool.disable_caching();

        params.log_writer.as_ref().inspect(|log| {
            log.add_header("Checking installation");
        });

        // Check if already installed, or if forced, overwrite previous install
        if !params.force && self.tool.is_setup(&spec).await? {
            self.pin_version(&spec, &params.pin_to).await?;
            self.finish_progress(started);

            return Ok(InstallOutcome::AlreadyInstalled(self.tool.get_id().clone()));
        }

        // Run pre-install hooks
        self.pre_install(&params).await?;

        // Run install
        let record = self.do_install(&spec, &params).await?;

        if record.is_none() {
            return Ok(InstallOutcome::FailedToInstall(self.tool.get_id().clone()));
        }

        let pinned = self.pin_version(&spec, &params.pin_to).await?;
        self.finish_progress(started);

        // Run post-install hooks
        self.post_install(&params).await?;

        // Track usage metrics
        track_usage(
            &self.tool.proto,
            Metric::InstallTool {
                id: self.tool.get_id().to_string(),
                plugin: self
                    .tool
                    .locator
                    .as_ref()
                    .map(|loc| loc.to_string())
                    .unwrap_or_default(),
                version: self.tool.get_resolved_version().to_string(),
                version_candidate: spec.req.to_string(),
                pinned,
            },
        )
        .await?;

        Ok(InstallOutcome::Installed(self.tool.get_id().clone()))
    }

    pub async fn install_with_logging(
        &mut self,
        spec: ToolSpec,
        mut params: InstallWorkflowParams,
    ) -> Result<InstallOutcome, ProtoCliError> {
        let log = match params.log_writer.clone() {
            Some(writer) => writer,
            None => {
                let writer = LogWriter::default();
                params.log_writer = Some(writer.clone());
                writer
            }
        };
        let log_path = self
            .tool
            .proto
            .working_dir
            .join(format!("proto-{}-install.log", self.tool.get_id()));

        // Capture plugin calls
        let on_log = log.clone();

        self.tool
            .plugin
            .set_on_call(Arc::new(move |func, input, output| {
                if let Some(input) = input {
                    on_log.add_subsection(format!("Plugin call: `{func}`"));
                    on_log.add_code("INPUT", input);
                } else if let Some(output) = output {
                    on_log.add_code("OUTPUT", output);
                }
            }));

        // Write a log based on result
        let is_build = self.is_build(params.strategy);
        let is_multiple = params.multiple;

        if is_build {
            log.add_title("Building from source");
        } else {
            log.add_title("Downloading pre-built");
        }

        match self.install(spec, params).await {
            Ok(record) => {
                let config = self.tool.proto.load_config()?;

                // Always write if configured to
                if is_build && config.settings.build.write_log_file {
                    log.write_to(log_path)?;
                }

                Ok(record)
            }
            Err(error) => {
                // Only write if an error and no direct UI
                if is_build || is_multiple {
                    log.add_title("Failure");
                    log.add_error(&error);
                    log.write_to(log_path)?;
                }

                Err(error)
            }
        }
    }

    async fn pre_install(&self, params: &InstallWorkflowParams) -> Result<(), ProtoCliError> {
        let tool = &self.tool;

        params.log_writer.as_ref().inspect(|log| {
            log.add_header("Running pre-install hooks");
        });

        unsafe { env::set_var("PROTO_INSTALL", tool.get_id().to_string()) };

        if tool.plugin.has_func(HookFunction::PreInstall).await {
            tool.plugin
                .call_func_without_output(
                    HookFunction::PreInstall,
                    InstallHook {
                        context: tool.create_plugin_context(),
                        forced: params.force,
                        passthrough_args: params.passthrough_args.clone(),
                        pinned: params.pin_to.is_some(),
                        quiet: params.quiet,
                    },
                )
                .await?;
        }

        Ok(())
    }

    async fn do_install(
        &mut self,
        spec: &ToolSpec,
        params: &InstallWorkflowParams,
    ) -> Result<Option<LockRecord>, ProtoCliError> {
        params.log_writer.as_ref().inspect(|log| {
            log.add_header("Installing tool");
        });

        self.tool.resolve_version(spec, false).await?;

        let resolved_version = self.tool.get_resolved_version();
        let default_strategy = self.tool.metadata.default_install_strategy;

        self.progress_reporter.set_message(
            if spec.req == resolved_version.to_unresolved_spec() {
                format!(
                    "Installing {} <version>{}</version>",
                    self.tool.get_name(),
                    resolved_version
                )
            } else {
                format!(
                    "Installing {} <version>{}</version> <mutedlight>(from specification <versionalt>{}</versionalt>)</mutedlight>",
                    self.tool.get_name(),
                    resolved_version,
                    spec
                )
            }
        );

        let pb = self.progress_reporter.clone();
        let on_download_chunk = Arc::new(move |current_bytes, total_bytes| {
            if current_bytes == 0 {
                pb.set_max(total_bytes);
            } else {
                pb.set_value(current_bytes);
            }
        });

        let pb = self.progress_reporter.clone();
        let on_phase_change = Arc::new(move |phase| {
            // Download phase is manually incremented based on streamed bytes,
            // while other phases are automatically ticked as a loader
            if matches!(phase, InstallPhase::Download { .. }) {
                pb.set_display(ProgressDisplay::Bar).set_tick(None);
            } else {
                pb.set_display(ProgressDisplay::Loader)
                    .set_tick(Some(Duration::from_millis(100)))
                    .set_max(100)
                    .set_value(0);
            }

            // Use the suffix for progress tokens so that they don't appear
            // in the normal message. This helps with non-TTY scenarios
            if matches!(phase, InstallPhase::Download { .. }) {
                pb.set_suffix(" <muted>|</muted> <mutedlight>{bytes} / {total_bytes}</mutedlight> <muted>|</muted> <shell>{bytes_per_sec}</shell>");
            } else {
                pb.set_suffix("");
            }

            pb.set_message(match phase {
                InstallPhase::Native => "Installing natively".to_owned(),
                InstallPhase::Verify { file, .. } => {
                    format!("Verifying checksum against <file>{file}</file>")
                }
                InstallPhase::Unpack { file } => format!("Unpacking archive <file>{file}</file>"),
                InstallPhase::Download { file, .. } => {
                    format!("Downloading pre-built archive <file>{file}</file>")
                }
                InstallPhase::InstallDeps => "Installing system dependencies".into(),
                InstallPhase::CheckRequirements => "Checking requirements".into(),
                InstallPhase::ExecuteInstructions => "Executing build instructions".into(),
                InstallPhase::CloneRepository { url } => {
                    format!("Cloning repository <url>{url}</url>")
                }
            });
        });

        let record = self
            .tool
            .setup(
                spec,
                InstallOptions {
                    console: Some(self.console.clone()),
                    on_download_chunk: Some(on_download_chunk),
                    on_phase_change: Some(on_phase_change),
                    force: params.force,
                    log_writer: params.log_writer.clone(),
                    skip_prompts: params.skip_prompts,
                    // When installing multiple tools, we can't render the nice
                    // UI for the build flow, so rely on the progress bars
                    skip_ui: params.multiple,
                    strategy: params.strategy.unwrap_or(default_strategy),
                },
            )
            .await?;

        Ok(record)
    }

    async fn post_install(&self, params: &InstallWorkflowParams) -> Result<(), ProtoCliError> {
        let tool = &self.tool;

        params.log_writer.as_ref().inspect(|log| {
            log.add_header("Running post-install hooks");
        });

        if tool.plugin.has_func(HookFunction::PostInstall).await {
            tool.plugin
                .call_func_without_output(
                    HookFunction::PostInstall,
                    InstallHook {
                        context: tool.create_plugin_context(),
                        forced: params.force,
                        passthrough_args: params.passthrough_args.clone(),
                        pinned: params.pin_to.is_some(),
                        quiet: params.quiet,
                    },
                )
                .await?;
        }

        self.update_shell(params).await?;

        Ok(())
    }

    async fn pin_version(
        &mut self,
        spec: &ToolSpec,
        arg_pin_to: &Option<PinLocation>,
    ) -> Result<bool, ProtoCliError> {
        let config = self.tool.proto.load_config()?;
        let mut pin_to = PinLocation::Local;
        let mut pin = false;

        // via `--pin` arg
        if let Some(custom_type) = arg_pin_to {
            pin_to = *custom_type;
            pin = true;
        }
        // Or the first time being installed
        else if !self.tool.inventory.dir.exists() {
            pin_to = PinLocation::Global;
            pin = true;
        }

        // via `pin-latest` setting
        if spec.req.is_latest()
            && let Some(custom_type) = &config.settings.pin_latest
        {
            pin_to = *custom_type;
            pin = true;
        }

        if pin {
            let resolved_spec =
                ToolSpec::new(self.tool.get_resolved_version().to_unresolved_spec());

            internal_pin(&self.tool.tool, &resolved_spec, pin_to).await?;
        }

        Ok(pin)
    }

    async fn update_shell(&self, params: &InstallWorkflowParams) -> Result<(), ProtoCliError> {
        let tool = &self.tool;

        if !tool.plugin.has_func(PluginFunction::SyncShellProfile).await {
            return Ok(());
        }

        let output: SyncShellProfileOutput = tool
            .plugin
            .call_func_with(
                PluginFunction::SyncShellProfile,
                SyncShellProfileInput {
                    context: tool.create_plugin_context(),
                    passthrough_args: params.passthrough_args.clone(),
                },
            )
            .await?;

        if output.skip_sync {
            return Ok(());
        }

        let shell_type = ShellType::try_detect()?;
        let shell = shell_type.build();

        debug!(
            shell = ?shell_type,
            exports = ?output.export_vars,
            paths = ?output.extend_path,
            "Updating shell profile",
        );

        let mut exports = vec![];

        if let Some(export_vars) = output.export_vars {
            for (key, value) in export_vars {
                exports.push(Export::Var(key, value));
            }
        }

        if let Some(extend_path) = output.extend_path {
            exports.push(Export::Path(extend_path));
        }

        if exports.is_empty() {
            return Ok(());
        }

        let profile_path = tool.proto.store.load_preferred_profile()?.and_then(|file| {
            if file.exists() {
                Some(file)
            } else {
                debug!(
                    profile = ?file,
                    "Configured shell profile path does not exist, will not use",
                );

                None
            }
        });

        if let Some(updated_profile) = profile_path {
            let exported_content = shell::format_exports(&shell, tool.get_id(), exports);

            if shell::update_profile_if_not_setup(
                &updated_profile,
                &exported_content,
                &output.check_var,
            )? {
                debug!(
                    profile = ?updated_profile,
                    "Added {} to shell profile",
                    output.check_var,
                );
            }
        }

        Ok(())
    }

    fn finish_progress(&mut self, started: Instant) {
        let duration = format_duration(started.elapsed(), true);
        let mut message = format!(
            "{} <version>{}</version> installed",
            self.tool.get_name(),
            self.tool.get_resolved_version(),
        );

        if duration != "0s" {
            message.push(' ');
            message.push_str(&format!("<mutedlight>({duration})</mutedlight>"));
        }

        self.progress_reporter
            .set_message(message)
            .set_display(ProgressDisplay::Bar)
            .set_tick(None)
            .set_max(100)
            .set_value(100);
    }
}

pub struct InstallWorkflowManager {
    pub console: ProtoConsole,
    pub progress_reporters: BTreeMap<Id, ProgressReporter>,
    pub quiet: bool,

    monitor_handles: Vec<JoinHandle<()>>,
    render_handle: Option<JoinHandle<Result<(), ConsoleError>>>,
}

impl InstallWorkflowManager {
    pub fn new(console: ProtoConsole, quiet: bool) -> Self {
        Self {
            console,
            progress_reporters: BTreeMap::default(),
            monitor_handles: vec![],
            render_handle: None,
            quiet,
        }
    }

    pub fn create_workflow(&mut self, tool: ToolRecord) -> InstallWorkflow {
        let workflow = InstallWorkflow::new(tool, self.console.clone());

        self.progress_reporters.insert(
            workflow.tool.get_id().clone(),
            workflow.progress_reporter.clone(),
        );

        workflow
    }

    pub fn is_tty(&self) -> bool {
        !bool_var("NO_TTY") && self.console.out.is_terminal()
    }

    pub async fn render_single_progress(&mut self) {
        if self.quiet {
            return;
        }

        if !self.is_tty() {
            self.monitor_messages();
            return;
        }

        let reporter = self.progress_reporters.values().next().unwrap().clone();
        let console = self.console.clone();

        self.render_handle = Some(tokio::spawn(async move {
            console
                .render_loop(element! {
                    InstallProgress(reporter)
                })
                .await
        }));

        // Wait a bit for the component to be rendered
        sleep(Duration::from_millis(50)).await;
    }

    pub async fn render_multiple_progress(&mut self) {
        if self.quiet {
            return;
        }

        if !self.is_tty() {
            self.monitor_messages();
            return;
        }

        let reporter = ProgressReporter::default();
        let reporter_clone = reporter.clone();
        let console = self.console.clone();

        let tools = self
            .progress_reporters
            .iter()
            .map(|(id, reporter)| {
                (
                    id.to_owned(),
                    InstallProgressProps {
                        default_message: Some("Preparing install…".into()),
                        reporter: Some(OwnedOrShared::Shared(Arc::new(reporter.clone()))),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();

        let id_width = self
            .progress_reporters
            .keys()
            .fold(0, |acc, id| acc.max(id.as_str().len()));

        self.render_handle = Some(tokio::spawn(async move {
            console
                .render_loop(element! {
                    InstallAllProgress(
                        reporter: reporter_clone,
                        tools,
                        id_width,
                    )
                })
                .await
        }));

        // Wait a bit for the component to be rendered
        sleep(Duration::from_millis(50)).await;
    }

    pub fn monitor_messages(&mut self) {
        for (id, reporter) in &self.progress_reporters {
            let reporter = reporter.clone();
            let console = self.console.clone();
            let prefix = color::muted_light(format!("[{id}] "));

            self.monitor_handles.push(tokio::spawn(async move {
                let mut receiver = reporter.subscribe();

                while let Ok(state) = receiver.recv().await {
                    match state {
                        ProgressState::Exit => {
                            break;
                        }
                        ProgressState::Message(message) => {
                            if !console.out.is_quiet() {
                                let _ = console.out.write_line_with_prefix(
                                    apply_style_tags(
                                        // Compatibility with the UI theme
                                        message
                                            .replace("version>", "hash>")
                                            .replace("versionalt>", "symbol>"),
                                    ),
                                    &prefix,
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }));
        }
    }

    pub async fn stop_rendering(mut self) -> miette::Result<()> {
        use miette::IntoDiagnostic;

        self.progress_reporters.values().for_each(|reporter| {
            reporter.exit();
        });

        for handle in self.monitor_handles {
            handle.await.into_diagnostic()?;
        }

        if let Some(handle) = self.render_handle.take() {
            handle.await.into_diagnostic()??;
        }

        Ok(())
    }
}
