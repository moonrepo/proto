use crate::commands::install::{InstallArgs, install_one};
use crate::error::ProtoCliError;
use crate::session::ProtoSession;
use clap::Args;
use miette::IntoDiagnostic;
use proto_core::flow::detect::ProtoDetectError;
use proto_core::{
    Id, PROTO_PLUGIN_KEY, ProtoConfigEnvOptions, ProtoEnvironment, ProtoLoaderError, Tool,
    ToolContext, ToolSpec,
};
use proto_pdk_api::{ExecutableConfig, HookFunction, RunHook, RunHookResult};
use proto_shim::{exec_command_and_replace, locate_proto_exe};
use starbase::AppResult;
use starbase_styles::color;
use starbase_utils::{
    env::{bool_var, paths},
    fs,
};
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
        hide = true,
        help = "Name of an alternate (secondary) binary to run"
    )]
    alt: Option<String>,

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
    bool_var("PROTO_AUTO_INSTALL_HIDE_OUTPUT")
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

async fn get_tool_executable(tool: &Tool, alt: Option<&str>) -> miette::Result<ExecutableConfig> {
    let tool_dir = tool.get_product_dir();

    // Run an alternate executable (via shim)
    if let Some(alt_name) = alt {
        for location in tool.resolve_shim_locations().await? {
            if location.name == alt_name {
                let Some(exe_path) = &location.config.exe_path else {
                    continue;
                };

                let alt_exe_path = tool_dir.join(exe_path);

                if alt_exe_path.exists() {
                    debug!(
                        bin = alt_name,
                        path = ?alt_exe_path,
                        "Received an alternate binary to run with",
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
            path: tool_dir,
        }
        .into());
    }

    // Otherwise use the primary
    let mut config = tool
        .resolve_primary_exe_location()
        .await?
        .expect("Required executable information missing!")
        .config;

    // We don't use `locate_exe_file` here because we need to handle
    // tools whose primary file is not executable, like JavaScript!
    config.exe_path = Some(tool_dir.join(config.exe_path.as_ref().unwrap()));

    Ok(config)
}

fn get_global_executable(env: &ProtoEnvironment, name: &str) -> Option<PathBuf> {
    let Ok(system_path) = env::var("PATH") else {
        return None;
    };

    let exe_name = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };

    for path_dir in env::split_paths(&system_path) {
        if path_dir.starts_with(&env.store.bin_dir) || path_dir.starts_with(&env.store.shims_dir) {
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

    let mut command = if let Some(parent_exe_path) = &exe_config.parent_exe_name {
        let mut args = exe_config
            .parent_exe_args
            .iter()
            .map(OsStr::new)
            .collect::<Vec<_>>();
        args.push(exe_path.as_os_str());
        args.extend(base_args.iter().map(|arg| arg.as_ref()));

        debug!(
            bin = ?parent_exe_path,
            args = ?args,
            pid = std::process::id(),
            "Running {}", tool.get_name(),
        );

        create_process_command(parent_exe_path, args)
    } else {
        let args = base_args.iter().map(|arg| arg.as_ref()).collect::<Vec<_>>();

        debug!(
            bin = ?exe_path,
            args = ?args,
            pid = std::process::id(),
            "Running {}", tool.get_name(),
        );

        create_process_command(exe_path, args)
    };

    for (key, value) in tool
        .proto
        .load_config()?
        .get_env_vars(ProtoConfigEnvOptions {
            check_process: true,
            include_shared: true,
            tool_id: Some(tool.get_id().clone()),
        })?
    {
        match value {
            Some(value) => {
                command.env(key, value);
            }
            None => {
                command.env_remove(key);
            }
        };
    }

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
pub async fn run(session: ProtoSession, args: RunArgs) -> AppResult {
    let mut tool = match session.load_tool(&args.context).await {
        Ok(tool) => tool,
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
    let spec = if use_global_proto {
        args.spec
            .clone()
            .unwrap_or_else(|| ToolSpec::parse("*").unwrap())
    } else if let Some(spec) = args.spec.clone() {
        spec
    } else {
        match tool.detect_version().await {
            Ok(spec) => spec,
            Err(error) => {
                return if matches!(error, ProtoDetectError::FailedVersionDetect { .. }) {
                    run_global_tool(session, args, error.into()).map(|_| None)
                } else {
                    Err(error.into())
                };
            }
        }
    };

    // Check if installed or need to install
    if tool.is_setup(&spec).await? {
        if tool.get_id() == PROTO_PLUGIN_KEY {
            use_global_proto = false;
        }
    } else {
        let config = tool.proto.load_config()?;
        let resolved_version = tool.get_resolved_version();

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
                        resolve_from_manifest: false,
                        read_lockfile: false,
                        write_lockfile: false,
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
                "No proto version detected or located, falling back to the global proto binary!"
            );
        }
        // Otherwise fail with a not installed error
        else {
            let command = format!("proto install {} {}", tool.context, resolved_version);

            if let Ok(source) = env::var(format!("{}_DETECTED_FROM", tool.get_env_var_prefix())) {
                return Err(ProtoCliError::RunMissingToolWithSource {
                    tool: tool.get_name().to_owned(),
                    version: spec.req.to_string(),
                    command,
                    path: source.into(),
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

    // Determine the binary path to execute
    let exe_config = if use_global_proto {
        ExecutableConfig {
            exe_path: locate_proto_exe("proto"),
            primary: true,
            ..Default::default()
        }
    } else {
        get_tool_executable(&tool, args.alt.as_deref()).await?
    };

    // Run before hook
    let hook_result = if tool.plugin.has_func(HookFunction::PreRun).await {
        let globals_dir = tool.locate_globals_dir().await?;
        let globals_prefix = tool.locate_globals_prefix().await?;

        // Ensure directory exists as some tools require it
        if let Some(dir) = &globals_dir {
            let _ = fs::create_dir_all(dir);
        }

        tool.plugin
            .call_func_with(
                HookFunction::PreRun,
                RunHook {
                    context: tool.create_plugin_context(),
                    globals_dir: globals_dir.map(|dir| tool.to_virtual_path(&dir)),
                    globals_prefix,
                    passthrough_args: args.passthrough.clone(),
                },
            )
            .await?
    } else {
        RunHookResult::default()
    };

    // Create and run the command
    let mut command = create_command(&tool, &exe_config, &args.passthrough)?;

    if let Some(hook_args) = hook_result.args {
        command.args(hook_args);
    }

    if let Some(hook_env) = hook_result.env {
        command.envs(hook_env);
    }

    if let Some(mut hook_paths) = hook_result.paths {
        hook_paths.extend(paths());
        command.env("PATH", env::join_paths(hook_paths).into_diagnostic()?);
    }

    if !use_global_proto {
        command
            .env(
                format!("{}_VERSION", tool.get_env_var_prefix()),
                tool.get_resolved_version().to_string(),
            )
            .env(
                format!("{}_BIN", tool.get_env_var_prefix()),
                exe_config.exe_path.as_ref().unwrap(),
            );
    }

    // Update the last used timestamp
    if env::var("PROTO_SKIP_USED_AT").is_err() {
        let _ = tool.product.track_used_at();
    }

    // Must be the last line!
    exec_command_and_replace(command)
        .into_diagnostic()
        .map(|_| None)
}
