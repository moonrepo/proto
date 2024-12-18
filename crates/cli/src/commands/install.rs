use super::pin::internal_pin;
use crate::error::ProtoCliError;
use crate::helpers::*;
use crate::session::ProtoSession;
use crate::shell::{self, Export};
use crate::telemetry::{track_usage, Metric};
use crate::utils::install_graph::*;
use clap::Args;
use indicatif::ProgressBar;
use iocraft::prelude::element;
use proto_core::flow::install::{InstallOptions, InstallPhase};
use proto_core::{
    ConfigMode, Id, PinLocation, Tool, UnresolvedVersionSpec, VersionSpec, PROTO_PLUGIN_KEY,
};
use proto_pdk_api::{InstallHook, SyncShellProfileInput, SyncShellProfileOutput};
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_shell::ShellType;
use starbase_styles::color;
use std::collections::BTreeMap;
use std::env;
use std::time::Duration;
use tokio::task::JoinSet;
use tokio::time::sleep;
use tracing::{debug, instrument, trace};

#[derive(Args, Clone, Debug, Default)]
pub struct InstallArgs {
    #[arg(help = "ID of a single tool to install")]
    pub id: Option<Id>,

    #[arg(
        default_value = "latest",
        help = "When installing one, the version or alias to install",
        group = "version-type"
    )]
    pub spec: Option<UnresolvedVersionSpec>,

    #[arg(
        long,
        help = "When installing one, use a canary (nightly, etc) version",
        group = "version-type"
    )]
    pub canary: bool,

    #[arg(
        long,
        help = "Force reinstallation of tool even if it is already installed"
    )]
    pub force: bool,

    #[arg(
        long,
        help = "When installing one, pin the resolved version to .prototools"
    )]
    pub pin: Option<Option<PinOption>>,

    // Passthrough args (after --)
    #[arg(
        last = true,
        help = "When installing one, additional arguments to pass to the tool"
    )]
    pub passthrough: Vec<String>,
}

impl InstallArgs {
    fn get_pin_location(&self) -> Option<PinLocation> {
        self.pin.as_ref().map(|pin| match pin {
            Some(PinOption::Global) => PinLocation::Global,
            Some(PinOption::User) => PinLocation::User,
            _ => PinLocation::Local,
        })
    }

    fn get_unresolved_spec(&self) -> UnresolvedVersionSpec {
        if self.canary {
            UnresolvedVersionSpec::Canary
        } else {
            self.spec.clone().unwrap_or_default()
        }
    }
}

pub fn enforce_requirements(
    tool: &Tool,
    versions: &BTreeMap<Id, UnresolvedVersionSpec>,
) -> miette::Result<()> {
    for require_id in &tool.metadata.requires {
        if !versions.contains_key(require_id.as_str()) {
            return Err(ProtoCliError::ToolRequiresNotMet {
                tool: tool.get_name().to_owned(),
                requires: require_id.to_owned(),
            }
            .into());
        }
    }

    Ok(())
}

async fn pin_version(
    tool: &mut Tool,
    initial_version: &UnresolvedVersionSpec,
    arg_pin_to: &Option<PinLocation>,
) -> miette::Result<bool> {
    // Don't pin the proto tool itself as it's internal only
    if tool.id.as_str() == PROTO_PLUGIN_KEY {
        return Ok(false);
    }

    let config = tool.proto.load_config()?;
    let spec = tool.get_resolved_version().to_unresolved_spec();
    let mut pin_to = PinLocation::Local;
    let mut pin = false;

    // via `--pin` arg
    if let Some(custom_type) = arg_pin_to {
        pin_to = *custom_type;
        pin = true;
    }
    // Or the first time being installed
    else if !tool.inventory.dir.exists() {
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
        internal_pin(tool, &spec, pin_to).await?;
    }

    Ok(pin)
}

async fn update_shell(tool: &Tool, passthrough_args: Vec<String>) -> miette::Result<()> {
    if !tool.plugin.has_func("sync_shell_profile").await {
        return Ok(());
    }

    let output: SyncShellProfileOutput = tool
        .plugin
        .call_func_with(
            "sync_shell_profile",
            SyncShellProfileInput {
                context: tool.create_context(),
                passthrough_args,
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
        env_vars = ?output.export_vars,
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
                "Added {} to shell profile {}",
                color::property(output.check_var),
                color::path(updated_profile)
            );
        }
    }

    Ok(())
}

#[instrument(skip_all)]
pub async fn do_install(
    tool: &mut Tool,
    args: InstallArgs,
    pb: &ProgressBar,
) -> miette::Result<bool> {
    let version = args.get_unresolved_spec();
    let pin_type = args.get_pin_location();
    let name = tool.get_name().to_owned();

    let finish_pb = |installed: bool, resolved_version: &VersionSpec| {
        print_progress_state(
            pb,
            if installed {
                format!("{name} {resolved_version} installed!")
            } else {
                color::muted_light(format!("{name} {resolved_version} already installed!"))
            },
        );

        if args.id.is_some() {
            pb.finish_and_clear();
        } else {
            pb.finish();
        }
    };

    // Disable version caching and always use the latest when installing
    tool.disable_caching();

    // Resolve version first so subsequent steps can reference the resolved version
    let resolved_version = tool.resolve_version(&version, false).await?;

    // Check if already installed, or if forced, overwrite previous install
    if !args.force && tool.is_setup(&version).await? {
        pin_version(tool, &version, &pin_type).await?;
        finish_pb(false, &resolved_version);

        return Ok(false);
    }

    // This ensures that the correct version is used by other processes
    env::set_var(
        format!("{}_VERSION", tool.get_env_var_prefix()),
        resolved_version.to_string(),
    );

    env::set_var("PROTO_INSTALL", tool.id.to_string());

    // Run before hook
    if tool.plugin.has_func("pre_install").await {
        tool.plugin
            .call_func_without_output(
                "pre_install",
                InstallHook {
                    context: tool.create_context(),
                    passthrough_args: args.passthrough.clone(),
                    pinned: pin_type.is_some(),
                },
            )
            .await?;
    }

    // Install the tool
    debug!(
        "Installing {} with version {} (from {})",
        tool.get_name(),
        resolved_version,
        version
    );

    let pb2 = pb.clone();
    let on_download_chunk = Box::new(move |current_bytes, total_bytes| {
        if current_bytes == 0 {
            pb2.set_length(total_bytes);
        } else {
            pb2.set_position(current_bytes);
        }
    });

    let pb3 = pb.clone();
    let on_phase_change = Box::new(move |phase| {
        pb3.reset();
        pb3.set_length(100);
        pb3.set_position(0);

        // Styles
        match phase {
            // Download is manually incremented based on streamed bytes
            InstallPhase::Download => {
                pb3.set_style(create_progress_bar_download_style());
                pb3.disable_steady_tick();
            }
            // Other phases are automatically ticked as a spinner
            _ => {
                pb3.set_style(create_progress_spinner_style());
                pb3.enable_steady_tick(Duration::from_millis(100));
            }
        };

        // Messages
        print_progress_state(
            &pb3,
            match phase {
                InstallPhase::Verify => "Verifying checksum",
                InstallPhase::Unpack => "Unpacking archive",
                InstallPhase::Download => "Downloading pre-built archive",
                _ => "",
            }
            .into(),
        );
    });

    let installed = tool
        .setup(
            &version,
            InstallOptions {
                on_download_chunk: Some(on_download_chunk),
                on_phase_change: Some(on_phase_change),
                force: args.force,
                ..InstallOptions::default()
            },
        )
        .await?;
    let pinned = pin_version(tool, &version, &pin_type).await?;

    finish_pb(installed, &resolved_version);

    if !installed {
        return Ok(false);
    }

    // Track usage metrics
    track_usage(
        &tool.proto,
        Metric::InstallTool {
            id: tool.id.to_string(),
            plugin: tool
                .locator
                .as_ref()
                .map(|loc| loc.to_string())
                .unwrap_or_default(),
            version: resolved_version.to_string(),
            version_candidate: version.to_string(),
            pinned,
        },
    )
    .await?;

    // Run after hook
    if tool.plugin.has_func("post_install").await {
        tool.plugin
            .call_func_without_output(
                "post_install",
                InstallHook {
                    context: tool.create_context(),
                    passthrough_args: args.passthrough.clone(),
                    pinned: pin_type.is_some(),
                },
            )
            .await?;
    }

    // Sync shell profile
    update_shell(tool, args.passthrough.clone()).await?;

    Ok(true)
}

#[instrument(skip(session, args))]
async fn install_one(session: &ProtoSession, id: &Id, args: InstallArgs) -> miette::Result<Tool> {
    debug!(id = id.as_str(), "Loading tool");

    let mut tool = session.load_tool(id).await?;

    // Load config including global versions,
    // so that our requirements can be satisfied
    let config = session.load_config_with_mode(ConfigMode::UpwardsGlobal)?;

    enforce_requirements(&tool, &config.versions)?;

    let pb = create_progress_bar(format!("Installing {}", tool.get_name()));

    if do_install(&mut tool, args, &pb).await? {
        session.console.render(element! {
            Notice(variant: Variant::Success) {
                StyledText(
                    content: format!(
                        "{} <hash>{}</hash> has been installed to <path>{}</path>!",
                        tool.get_name(),
                        tool.get_resolved_version(),
                        tool.get_product_dir().display(),
                    ),
                )
            }
        })?;
    } else {
        session.console.render(element! {
            Notice(variant: Variant::Info) {
                StyledText(
                    content: format!(
                        "{} <hash>{}</hash> has already been installed at <path>{}</path>!",
                        tool.get_name(),
                        tool.get_resolved_version(),
                        tool.get_product_dir().display(),
                    ),
                )
            }
        })?;
    }

    Ok(tool.tool)
}

#[instrument(skip_all)]
pub async fn install_all(session: &ProtoSession) -> AppResult {
    debug!("Loading all tools");

    let tools = session.load_tools().await?;

    debug!("Detecting tool versions to install");

    let mut versions = session.load_config()?.versions.to_owned();
    versions.remove(PROTO_PLUGIN_KEY);

    for tool in &tools {
        if versions.contains_key(&tool.id) {
            continue;
        }

        if let Some((candidate, _)) = tool.detect_version_from(&session.env.cwd).await? {
            debug!("Detected version {} for {}", candidate, tool.get_name());

            versions.insert(tool.id.clone(), candidate);
        }
    }

    if versions.is_empty() {
        session.console.render(element! {
            Notice(variant: Variant::Caution) {
                StyledText(
                    content: "No versions have been configured, nothing to install!",
                )
                #(if session.env.config_mode == ConfigMode::UpwardsGlobal {
                    None
                } else {
                    Some(element! {
                        View(margin_top: 1) {
                            StyledText(
                                content: format!(
                                    "Configuration has been loaded in <symbol>{}</symbol> mode. Try changing the mode with <property>--config-mode</property> to include other pinned versions.",
                                    session.env.config_mode
                                )
                            )
                        }
                    })
                })
            }
        })?;

        return Ok(Some(1));
    }

    // Filter down tools to only those that have a version
    let tools = tools
        .into_iter()
        .filter(|tool| versions.contains_key(&tool.id))
        .collect::<Vec<_>>();

    // Determine longest ID for use within progress bars
    let longest_id = versions
        .keys()
        .fold(0, |acc, id| if id.len() > acc { id.len() } else { acc });

    // Then install each tool in parallel!
    let mpb = create_multi_progress_bar();
    let pbs = create_progress_bar_style();
    let graph = InstallGraph::new(&tools);
    let mut set = JoinSet::new();

    for mut tool in tools {
        enforce_requirements(&tool, &versions)?;

        if let Some(version) = versions.get(&tool.id) {
            let tool_id = tool.id.clone();
            let version = version.clone();
            let graph = graph.clone();

            let pb = mpb.add(ProgressBar::new(0));
            pb.set_style(pbs.clone());

            // Defer writing content till the thread starts,
            // otherwise the progress bars fail to render correctly
            let handle = set.spawn(async move {
                sleep(Duration::from_millis(25)).await;

                pb.set_prefix(color::id(format!(
                    "{}{}",
                    " ".repeat(longest_id - tool.id.len()),
                    tool.id
                )));

                while let Some(status) = graph.check_install_status(&tool.id).await {
                    match status {
                        InstallStatus::ReqFailed(req_id) => {
                            print_progress_state(
                                &pb,
                                format!("Requirement {} failed to install", req_id),
                            );

                            return false;
                        }
                        InstallStatus::WaitingOnReqs(waiting_on) => {
                            print_progress_state(
                                &pb,
                                format!("Waiting on requirements: {}", waiting_on.join(", ")),
                            );
                        }
                    };

                    sleep(Duration::from_millis(150)).await;
                }

                print_progress_state(&pb, format!("Installing {} {}", tool.get_name(), version));

                match do_install(
                    &mut tool,
                    InstallArgs {
                        spec: Some(version),
                        ..Default::default()
                    },
                    &pb,
                )
                .await
                {
                    Ok(_) => {
                        graph.mark_installed(&tool.id).await;
                        true
                    }
                    Err(error) => {
                        print_progress_state(&pb, error.to_string());
                        graph.mark_not_installed(&tool.id).await;
                        false
                    }
                }
            });

            trace!(
                task_id = handle.id().to_string(),
                "Spawning {} in background task",
                color::id(tool_id)
            );
        }
    }

    let mut installed_count = 0;
    let mut failed_count = 0;

    while let Some(result) = set.join_next_with_id().await {
        match result {
            Err(error) => {
                trace!(
                    task_id = error.id().to_string(),
                    "Spawned task failed: {}",
                    error
                );
                failed_count += 1;
            }
            Ok((task_id, installed)) => {
                trace!(task_id = task_id.to_string(), "Spawned task successful");

                if installed {
                    installed_count += 1;
                } else {
                    failed_count += 1;
                }
            }
        };
    }

    // When no TTY, we should display something to the user!
    if mpb.is_hidden() {
        if installed_count > 0 {
            session
                .console
                .out
                .write_line(format!("Successfully installed {} tools!", installed_count))?;
        }

        if failed_count > 0 {
            session
                .console
                .out
                .write_line(format!("Failed to install {} tools!", failed_count))?;
        }
    }

    Ok(None)
}

#[instrument(skip_all)]
pub async fn install(session: ProtoSession, args: InstallArgs) -> AppResult {
    match args.id.clone() {
        Some(id) => {
            install_one(&session, &id, args).await?;
        }
        None => {
            install_all(&session).await?;
        }
    };

    Ok(None)
}
