use crate::commands::install::{InstallArgs, install_one};
use crate::error::ProtoCliError;
use crate::session::ProtoSession;
use crate::workflows::{ExecWorkflow, ExecWorkflowParams};
use clap::Args;
use miette::IntoDiagnostic;
use proto_core::flow::detect::{Detector, ProtoDetectError};
use proto_core::flow::locate::{Locator, ProtoLocateError};
use proto_core::flow::resolve::Resolver;
use proto_core::layout::ShimRegistry;
use proto_core::{
    Id, PROTO_PLUGIN_KEY, ProtoEnvironment, ProtoLoaderError, Tool, ToolContext, ToolSpec,
};
use proto_pdk_api::ExecutableConfig;
use proto_shim::{exec_command_and_replace, locate_proto_exe};
use rustc_hash::FxHashMap;
use starbase::AppResult;
use starbase_styles::color;
use starbase_utils::{envx, path};
use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;
use system_env::create_process_command;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct RunArgs {
    #[arg(required = true, help = "Tool to run")]
    context: ToolContext,

    #[arg(help = "Version specification to run")]
    spec: Option<ToolSpec>,

    #[arg(
        long,
        alias = "alt",
        help = "File name of an alternate (secondary) executable to run"
    )]
    exe: Option<String>,

    // Passthrough args (after --)
    #[arg(
        last = true,
        help = "Arguments to pass through to the underlying command"
    )]
    passthrough: Vec<String>,
}

fn should_use_global_proto(tool: &Tool) -> miette::Result<bool> {
    if tool.get_id() != PROTO_PLUGIN_KEY {
        return Ok(false);
    }

    let config = tool.proto.load_config()?;
    let proto_context = ToolContext::new(Id::raw(PROTO_PLUGIN_KEY));

    Ok(
        // No pinnned version
        !config.versions.contains_key(&proto_context)
        // Pinned but the same as the running process
        || config.versions.get(&proto_context).is_some_and(|v| v.req.to_string() == env!("CARGO_PKG_VERSION")),
    )
}

fn should_hide_auto_install_output(args: &[String]) -> bool {
    envx::bool_var("PROTO_AUTO_INSTALL_HIDE_OUTPUT")
        || args.iter().any(|arg| arg == "--version" || arg == "--help")
}

fn is_trying_to_self_upgrade(tool: &Tool, args: &[String]) -> bool {
    if tool.get_id() == PROTO_PLUGIN_KEY
        || tool.metadata.self_upgrade_commands.is_empty()
        || args.is_empty()
    {
        return false;
    }

    // Expand "self upgrade" string into ["self", "upgrade"] list
    let mut match_groups = vec![];

    for arg_string in &tool.metadata.self_upgrade_commands {
        if let Ok(arg_list) = shell_words::split(arg_string) {
            match_groups.push(arg_list);
        }
    }

    // Then match the args in sequence
    'outer: for match_list in match_groups {
        for (index, match_arg) in match_list.into_iter().enumerate() {
            if args.get(index).is_some_and(|arg| arg != &match_arg) {
                continue 'outer;
            }
        }

        return true;
    }

    false
}

async fn get_tool_executable(
    tool: &Tool,
    spec: &ToolSpec,
    alt: Option<&str>,
) -> miette::Result<ExecutableConfig> {
    let locator = Locator::new(tool, spec);

    // Run an alternate executable (via shim)
    if let Some(alt_name) = alt {
        for location in locator.locate_shims().await? {
            if location.name == alt_name {
                let Some(exe_path) = &location.config.exe_path else {
                    continue;
                };

                let alt_exe_path = locator.product_dir.join(exe_path);

                if alt_exe_path.exists() {
                    debug!(
                        exe = alt_name,
                        path = ?alt_exe_path,
                        "Received an alternate executable to run with",
                    );

                    return Ok(ExecutableConfig {
                        exe_path: Some(alt_exe_path),
                        ..location.config
                    });
                }
            }
        }

        return Err(ProtoCliError::RunMissingAltBin {
            bin: alt_name.to_owned(),
            path: locator.product_dir.clone(),
        }
        .into());
    }

    // Otherwise use the primary
    let mut config = match locator.locate_primary_exe().await? {
        Some(inner) => inner.config,
        None => {
            return Err(ProtoLocateError::NoPrimaryExecutable {
                tool: tool.get_name().into(),
            }
            .into());
        }
    };

    // We don't use `locate_exe_file` here because we need to handle
    // tools whose primary file is not executable, like JavaScript!
    config.exe_path = Some(locator.product_dir.join(config.exe_path.as_ref().unwrap()));

    Ok(config)
}

fn get_global_executable(env: &ProtoEnvironment, name: &str) -> Option<PathBuf> {
    let Ok(system_path) = env::var("PATH") else {
        return None;
    };

    let exe_name = path::exe_name(name);

    for path_dir in env::split_paths(&system_path) {
        if path_dir.starts_with(&env.store.bin_dir) || path_dir.starts_with(&env.store.shims_dir) {
            continue;
        }

        // Local development may have ~/.proto on PATH, so ignore!
        #[cfg(debug_assertions)]
        if path_dir.to_string_lossy().contains(".proto") {
            continue;
        }

        let path = path_dir.join(&exe_name);

        if path.exists() && path.is_file() {
            return Some(path);
        }
    }

    None
}

fn create_command<I: IntoIterator<Item = A>, A: AsRef<OsStr>>(
    tool: &Tool,
    exe_config: &ExecutableConfig,
    args: I,
) -> miette::Result<Command> {
    let exe_path = exe_config
        .exe_path
        .as_ref()
        .expect("Could not determine executable path.");
    let base_args = args.into_iter().collect::<Vec<_>>();

    let command = if let Some(parent_exe_path) = &exe_config.parent_exe_name {
        let mut args = exe_config
            .parent_exe_args
            .iter()
            .map(OsStr::new)
            .collect::<Vec<_>>();
        args.push(exe_path.as_os_str());
        args.extend(base_args.iter().map(|arg| arg.as_ref()));

        debug!(
            exe = ?parent_exe_path,
            args = ?args,
            pid = std::process::id(),
            "Running {}", tool.get_name(),
        );

        create_process_command(parent_exe_path, args)
    } else {
        let args = base_args.iter().map(|arg| arg.as_ref()).collect::<Vec<_>>();

        debug!(
            exe = ?exe_path,
            args = ?args,
            pid = std::process::id(),
            "Running {}", tool.get_name(),
        );

        create_process_command(exe_path, args)
    };

    Ok(command)
}

// It is possible that we have a shim for the tool, but can not find the
// plugin or version. However, this tool may exist on `PATH` outside
// of proto, so try and fallback to it!
fn run_global_tool(
    session: ProtoSession,
    args: RunArgs,
    error: miette::Report,
) -> miette::Result<()> {
    if let Some(global_exe) = get_global_executable(&session.env, args.context.id.as_str()) {
        debug!(
            global_exe = ?global_exe,
            "Tool {} is currently not managed by proto but exists on PATH, falling back to the global executable",
            color::shell(args.context.id),
        );

        return exec_command_and_replace(create_process_command(global_exe, args.passthrough))
            .into_diagnostic();
    }

    Err(error)
}

#[tracing::instrument(skip_all)]
pub async fn run(session: ProtoSession, mut args: RunArgs) -> AppResult {
    let tool = match session.load_tool(&args.context).await {
        Ok(tool) => tool,
        Err(ProtoLoaderError::UnknownTool { id }) => {
            // Check if this is a bin provided by another tool (e.g., `npx` from `npm`).
            // The shims registry contains mappings of secondary bins to their parent tools,
            // which is maintained by proto during tool installation.
            debug!(
                bin = id.as_str(),
                "Tool not found, checking shims registry for bin-to-tool mapping"
            );

            let registry = ShimRegistry::load(&session.env.store.shims_dir)?;
            let mut parent_tool_id: Option<Id> = None;
            let mut before_args: Vec<String> = vec![];
            let mut after_args: Vec<String> = vec![];

            // Try reading the shims registry
            if let Some(shim_entry) = registry.shims.get(id.as_str())
                && let Some(parent) = &shim_entry.parent
            {
                debug!(
                    bin = id.as_str(),
                    parent_tool = parent,
                    "Found {} in shims registry, redirecting to {}",
                    id.as_str(),
                    parent
                );

                parent_tool_id = Some(Id::raw(parent));

                // Store before/after args from the shim entry
                before_args = shim_entry.before_args.clone();
                after_args = shim_entry.after_args.clone();
            }

            if let Some(parent_id) = parent_tool_id {
                // Update args to run the parent tool with this bin as an alternate executable
                args.exe = Some(id.to_string());
                args.context = ToolContext::new(parent_id);

                // Prepend before_args and append after_args to passthrough
                let mut new_passthrough = before_args;
                new_passthrough.extend(args.passthrough.clone());
                new_passthrough.extend(after_args);
                args.passthrough = new_passthrough;

                // Load the parent tool (this will handle auto-install if enabled,
                // or show a proper error message if the tool is not installed)
                session.load_tool(&args.context).await?
            } else {
                // Not found in shims registry, fall back to global tool on PATH
                return run_global_tool(session, args, ProtoLoaderError::UnknownTool { id }.into())
                    .map(|_| None);
            }
        }
        Err(error) => {
            return if matches!(error, ProtoLoaderError::UnknownTool { .. }) {
                run_global_tool(session, args, error.into()).map(|_| None)
            } else {
                Err(error.into())
            };
        }
    };

    let mut use_global_proto = should_use_global_proto(&tool)?;

    // Avoid running the tool's native self-upgrade as it conflicts with proto
    if is_trying_to_self_upgrade(&tool, &args.passthrough) {
        return Err(ProtoCliError::RunNoSelfUpgrade {
            command: format!("proto install {} latest --pin", tool.context),
            tool: tool.get_name().to_owned(),
        }
        .into());
    }

    // Detect a version to run with
    let (mut spec, detected_source) = if use_global_proto {
        (
            args.spec
                .clone()
                .unwrap_or_else(|| ToolSpec::parse("*").unwrap()),
            None,
        )
    } else if let Some(spec) = args.spec.clone() {
        (spec, None)
    } else {
        match Detector::detect(&tool).await {
            Ok((spec, source)) => (spec, source),
            Err(error) => {
                return if matches!(error, ProtoDetectError::FailedVersionDetect { .. }) {
                    run_global_tool(session, args, error.into()).map(|_| None)
                } else {
                    Err(error.into())
                };
            }
        }
    };

    Resolver::resolve(&tool, &mut spec, true).await?;

    // Check if installed or need to install
    if tool.is_installed(&spec) {
        if tool.get_id() == PROTO_PLUGIN_KEY {
            use_global_proto = false;
        }
    } else {
        let config = tool.proto.load_config()?;
        let resolved_version = spec.get_resolved_version();

        // Auto-install the missing tool
        if config.settings.auto_install {
            let hide_output = should_hide_auto_install_output(&args.passthrough);

            if hide_output {
                session.console.set_quiet(true);
            } else {
                session.console.out.write_line(format!(
                    "Auto-install is enabled, attempting to install {} {}",
                    tool.get_name(),
                    resolved_version,
                ))?;
            }

            install_one(
                session.clone(),
                InstallArgs {
                    internal: true,
                    quiet: hide_output,
                    spec: Some(ToolSpec {
                        req: resolved_version.to_unresolved_spec(),
                        version: Some(resolved_version.clone()),
                        version_locked: None,
                        resolve_from_manifest: false,
                        resolve_from_lockfile: false,
                        update_lockfile: false,
                    }),
                    ..Default::default()
                },
                tool.context.clone(),
            )
            .await?;

            if hide_output {
                session.console.set_quiet(false);
            } else {
                session.console.out.write_line(format!(
                    "{} {} has been installed, continuing execution...",
                    tool.get_name(),
                    resolved_version,
                ))?;
            }
        }
        // If this is the proto tool running, continue instead of failing
        else if use_global_proto {
            debug!(
                "No proto version detected or located, falling back to the global proto executable!"
            );
        }
        // Otherwise fail with a not installed error
        else {
            let command = format!("proto install {} {}", tool.context, resolved_version);

            if let Some(source) = detected_source {
                return Err(ProtoCliError::RunMissingToolWithSource {
                    tool: tool.get_name().to_owned(),
                    version: spec.req.to_string(),
                    command,
                    path: source,
                }
                .into());
            }

            return Err(ProtoCliError::RunMissingTool {
                tool: tool.get_name().to_owned(),
                version: spec.req.to_string(),
                command,
            }
            .into());
        }
    }

    // Determine the executable path to execute and create command
    let exe_config = if use_global_proto {
        ExecutableConfig {
            exe_path: locate_proto_exe("proto"),
            primary: true,
            ..Default::default()
        }
    } else {
        get_tool_executable(&tool, &spec, args.exe.as_deref()).await?
    };

    let mut command = create_command(&tool, &exe_config, &args.passthrough)?;

    // Prepare environment
    let config = session.load_config()?;
    let context = tool.context.clone();
    let mut workflow = ExecWorkflow::new(vec![tool], config);

    workflow
        .prepare_environment(
            FxHashMap::from_iter([(context, spec)]),
            ExecWorkflowParams {
                activate_environment: true,
                check_process_env: true,
                passthrough_args: args.passthrough,
                pre_run_hook: true,
                version_env_vars: !use_global_proto,
                ..Default::default()
            },
        )
        .await?;

    workflow.apply_to_command(&mut command, false)?;

    // Must be the last line!
    exec_command_and_replace(command)
        .into_diagnostic()
        .map(|_| None)
}
