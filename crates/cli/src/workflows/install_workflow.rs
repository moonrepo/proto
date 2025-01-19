use crate::commands::pin::internal_pin;
use crate::session::ProtoConsole;
use crate::shell::{self, Export};
use crate::telemetry::*;
use crate::utils::tool_record::ToolRecord;
use proto_core::flow::install::{InstallOptions, InstallPhase, InstallStrategy};
use proto_core::{PinLocation, UnresolvedVersionSpec, PROTO_PLUGIN_KEY};
use proto_pdk_api::{InstallHook, SyncShellProfileInput, SyncShellProfileOutput};
use starbase_console::ui::{ProgressDisplay, ProgressReporter};
use starbase_console::utils::formats::format_duration;
use starbase_shell::ShellType;
use std::env;
use std::time::{Duration, Instant};
use tracing::debug;

pub enum InstallOutcome {
    AlreadyInstalled,
    Installed,
    FailedToInstall,
}

#[derive(Default)]
pub struct InstallWorkflowParams {
    pub build: bool,
    pub force: bool,
    pub multiple: bool,
    pub passthrough_args: Vec<String>,
    pub pin_to: Option<PinLocation>,
}

pub struct InstallWorkflow {
    pub console: ProtoConsole,
    pub progress_reporter: ProgressReporter,
    pub tool: ToolRecord,
}

impl InstallWorkflow {
    pub fn new(tool: ToolRecord, console: ProtoConsole) -> Self {
        Self {
            console,
            progress_reporter: ProgressReporter::default(),
            tool,
        }
    }

    pub async fn install(
        &mut self,
        initial_version: UnresolvedVersionSpec,
        params: InstallWorkflowParams,
    ) -> miette::Result<InstallOutcome> {
        let started = Instant::now();

        self.progress_reporter.set_message(format!(
            "Installing {} with specification <versionalt>{}</versionalt>",
            self.tool.get_name(),
            initial_version
        ));

        // Disable version caching and always use the latest when installing
        self.tool.disable_caching();

        // Check if already installed, or if forced, overwrite previous install
        if !params.force && self.tool.is_setup(&initial_version).await? {
            self.pin_version(&initial_version, &params.pin_to).await?;
            self.finish_progress(started);

            return Ok(InstallOutcome::AlreadyInstalled);
        }

        // Run pre-install hooks
        self.pre_install(&params).await?;

        // Run install
        let installed = self.do_install(&initial_version, &params).await?;

        if !installed {
            return Ok(InstallOutcome::FailedToInstall);
        }

        let pinned = self.pin_version(&initial_version, &params.pin_to).await?;
        self.finish_progress(started);

        // Run post-install hooks
        self.post_install(&params).await?;

        // Track usage metrics
        track_usage(
            &self.tool.proto,
            Metric::InstallTool {
                id: self.tool.id.to_string(),
                plugin: self
                    .tool
                    .locator
                    .as_ref()
                    .map(|loc| loc.to_string())
                    .unwrap_or_default(),
                version: self.tool.get_resolved_version().to_string(),
                version_candidate: initial_version.to_string(),
                pinned,
            },
        )
        .await?;

        Ok(InstallOutcome::Installed)
    }

    async fn pre_install(&self, params: &InstallWorkflowParams) -> miette::Result<()> {
        let tool = &self.tool;

        env::set_var(
            format!("{}_VERSION", tool.get_env_var_prefix()),
            tool.get_resolved_version().to_string(),
        );

        env::set_var("PROTO_INSTALL", tool.id.to_string());

        if tool.plugin.has_func("pre_install").await {
            tool.plugin
                .call_func_without_output(
                    "pre_install",
                    InstallHook {
                        context: tool.create_context(),
                        passthrough_args: params.passthrough_args.clone(),
                        pinned: params.pin_to.is_some(),
                    },
                )
                .await?;
        }

        Ok(())
    }

    async fn do_install(
        &mut self,
        initial_version: &UnresolvedVersionSpec,
        params: &InstallWorkflowParams,
    ) -> miette::Result<bool> {
        self.tool.resolve_version(initial_version, false).await?;

        let resolved_version = self.tool.get_resolved_version();

        self.progress_reporter.set_message(
            if initial_version == &resolved_version.to_unresolved_spec() {
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
                    initial_version
                )
            }
        );

        let pb = self.progress_reporter.clone();
        let on_download_chunk = Box::new(move |current_bytes, total_bytes| {
            if current_bytes == 0 {
                pb.set_max(total_bytes);
            } else {
                pb.set_value(current_bytes);
            }
        });

        let pb = self.progress_reporter.clone();
        let on_phase_change = Box::new(move |phase| {
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

            pb.set_message(match phase {
                InstallPhase::Native => "Installing natively".to_owned(),
                InstallPhase::Verify { file, .. } => format!("Verifying checksum against <file>{file}</file>"),
                InstallPhase::Unpack { file } => format!("Unpacking archive <file>{file}</file>"),
                InstallPhase::Download { file, .. } => format!("Downloading pre-built archive <file>{file}</file> <muted>|</muted> <mutedlight>{{bytes}} / {{total_bytes}}</mutedlight> <muted>|</muted> <shell>{{bytes_per_sec}}</shell>"),
                InstallPhase::InstallDeps => "Installing system depedencies".into(),
                InstallPhase::CheckRequirements => "Checking requirements".into(),
                InstallPhase::ExecuteInstructions => "Executing build instructions".into(),
                InstallPhase::CloneRepository { url } => format!("Cloning repository <url>{url}</url>")
            });
        });

        self.tool
            .setup(
                initial_version,
                InstallOptions {
                    console: if params.multiple {
                        None
                    } else {
                        Some(self.console.clone())
                    },
                    on_download_chunk: Some(on_download_chunk),
                    on_phase_change: Some(on_phase_change),
                    force: params.force,
                    strategy: if params.build {
                        InstallStrategy::BuildFromSource
                    } else {
                        InstallStrategy::DownloadPrebuilt
                    },
                    ..InstallOptions::default()
                },
            )
            .await
    }

    async fn post_install(&self, params: &InstallWorkflowParams) -> miette::Result<()> {
        let tool = &self.tool;

        if tool.plugin.has_func("post_install").await {
            tool.plugin
                .call_func_without_output(
                    "post_install",
                    InstallHook {
                        context: tool.create_context(),
                        passthrough_args: params.passthrough_args.clone(),
                        pinned: params.pin_to.is_some(),
                    },
                )
                .await?;
        }

        self.update_shell(params).await?;

        Ok(())
    }

    async fn pin_version(
        &mut self,
        initial_version: &UnresolvedVersionSpec,
        arg_pin_to: &Option<PinLocation>,
    ) -> miette::Result<bool> {
        // Don't pin the proto tool itself as it's internal only
        if self.tool.id.as_str() == PROTO_PLUGIN_KEY {
            return Ok(false);
        }

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
        if initial_version.is_latest() {
            if let Some(custom_type) = &config.settings.pin_latest {
                pin_to = *custom_type;
                pin = true;
            }
        }

        if pin {
            let resolved_version = self.tool.get_resolved_version();

            internal_pin(
                &mut self.tool.tool,
                &resolved_version.to_unresolved_spec(),
                pin_to,
            )
            .await?;
        }

        Ok(pin)
    }

    async fn update_shell(&self, params: &InstallWorkflowParams) -> miette::Result<()> {
        let tool = &self.tool;

        if !tool.plugin.has_func("sync_shell_profile").await {
            return Ok(());
        }

        let output: SyncShellProfileOutput = tool
            .plugin
            .call_func_with(
                "sync_shell_profile",
                SyncShellProfileInput {
                    context: tool.create_context(),
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
            let exported_content = shell::format_exports(&shell, &tool.id, exports);

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

    fn finish_progress(&self, started: Instant) {
        self.progress_reporter
            .set_message(format!(
                "{} <version>{}</version> installed <mutedlight>({})</mutedlight>!",
                self.tool.get_name(),
                self.tool.get_resolved_version(),
                format_duration(started.elapsed(), true)
            ))
            .set_display(ProgressDisplay::Bar)
            .set_tick(None)
            .set_max(100)
            .set_value(100);
    }
}
