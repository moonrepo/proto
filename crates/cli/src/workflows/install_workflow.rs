use crate::commands::pin::internal_pin;
use crate::shell::{self, Export};
use crate::telemetry::*;
use crate::utils::tool_record::ToolRecord;
use proto_core::flow::install::{InstallOptions, InstallPhase};
use proto_core::{PinLocation, UnresolvedVersionSpec, PROTO_PLUGIN_KEY};
use proto_pdk_api::{InstallHook, SyncShellProfileInput, SyncShellProfileOutput};
use starbase_console::ui::ProgressReporter;
use starbase_shell::ShellType;
use std::env;
use std::time::Duration;
use tokio::sync::broadcast::{self, Receiver, Sender};
use tracing::debug;

pub enum InstallOutcome {
    AlreadyInstalled,
    Installed,
    FailedToInstall,
}

#[derive(Default)]
pub struct InstallWorkflowParams {
    pub force: bool,
    pub passthrough_args: Vec<String>,
    pub pin_to: Option<PinLocation>,
}

pub struct InstallWorkflow {
    pub phase_reporter: InstallPhaseReporter,
    pub progress_reporter: ProgressReporter,
    pub tool: ToolRecord,
}

impl InstallWorkflow {
    pub fn new(tool: ToolRecord) -> Self {
        Self {
            phase_reporter: InstallPhaseReporter::default(),
            progress_reporter: ProgressReporter::default(),
            tool,
        }
    }

    pub async fn install(
        &mut self,
        initial_version: UnresolvedVersionSpec,
        params: InstallWorkflowParams,
    ) -> miette::Result<InstallOutcome> {
        self.progress_reporter.set_message(format!(
            "Installing {} with specification <symbol>{}</symbol>",
            self.tool.get_name(),
            initial_version
        ));

        // Disable version caching and always use the latest when installing
        self.tool.disable_caching();

        // Check if already installed, or if forced, overwrite previous install
        if !params.force && self.tool.is_setup(&initial_version).await? {
            self.pin_version(&initial_version, &params.pin_to).await?;
            self.finish_progress(false);

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
        self.finish_progress(true);

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
        let resolved_version = self.tool.get_resolved_version();

        if resolved_version.to_string() == initial_version.to_string() {
            self.progress_reporter.set_message(format!(
                "Installing {} <hash>{}</hash>",
                self.tool.get_name(),
                resolved_version
            ));
        } else {
            self.progress_reporter.set_message(format!(
                "Installing {} <hash>{}</hash> <mutedlight>(from specification <symbol>{}</symbol>)</mutedlight>",
                self.tool.get_name(),
                resolved_version,
                initial_version
            ));
        }

        let ph = self.phase_reporter.clone();
        let pb = self.progress_reporter.clone();
        let on_download_chunk = Box::new(move |current_bytes, total_bytes| {
            if current_bytes == total_bytes {
                // Do not set to 100%, otherwise the progress bar
                // will immediately exit and stop rendering
                pb.set_value(total_bytes - 1);

                // Also trigger the next phase a bit early so that
                // the component re-renders to a loader faster
                ph.set(InstallPhase::Verify);
                pb.wait(Duration::from_millis(25));
            } else if current_bytes == 0 {
                pb.set_max(total_bytes);
            } else {
                pb.set_value(current_bytes);
            }
        });

        let ph = self.phase_reporter.clone();
        let pb = self.progress_reporter.clone();
        let on_phase_change = Box::new(move |phase| {
            ph.set(phase);

            // Download phase is manually incremented based on streamed bytes,
            // while other phases are automatically ticked as a loader
            if !matches!(phase, InstallPhase::Download) {
                pb.set_max(100);
                pb.set_value(0);
            }

            let message = match phase {
                InstallPhase::Verify => "Verifying checksum",
                InstallPhase::Unpack => "Unpacking archive",
                InstallPhase::Download => "Downloading pre-built archive <muted>|</muted> <mutedlight>{bytes} / {total_bytes}</mutedlight> <muted>|</muted> <shell>{bytes_per_sec}</shell>",
                _ => return,
            };

            pb.set_message(message);
        });

        self.tool
            .setup(
                initial_version,
                InstallOptions {
                    on_download_chunk: Some(on_download_chunk),
                    on_phase_change: Some(on_phase_change),
                    force: params.force,
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

    fn finish_progress(&self, installed: bool) {
        let message = format!(
            "{} <hash>{}</hash> installed!",
            self.tool.get_name(),
            self.tool.get_resolved_version()
        );

        self.progress_reporter.set_message(if installed {
            message
        } else {
            format!("<mutedlight>{message}</mutedlight>")
        });
    }
}

#[derive(Clone)]
pub struct InstallPhaseReporter {
    tx: Sender<InstallPhase>,
}

impl Default for InstallPhaseReporter {
    fn default() -> Self {
        let (tx, _rx) = broadcast::channel::<InstallPhase>(1000);

        Self { tx }
    }
}

impl InstallPhaseReporter {
    pub fn subscribe(&self) -> Receiver<InstallPhase> {
        self.tx.subscribe()
    }

    pub fn set(&self, phase: InstallPhase) {
        let _ = self.tx.send(phase);
    }
}
