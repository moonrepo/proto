use crate::commands::install::{install_one, InstallArgs};
use crate::error::ProtoCliError;
use crate::session::ProtoSession;
use clap::Args;
use miette::IntoDiagnostic;
use proto_core::{detect_version_with_spec, Id, Tool, ToolSpec};
use proto_pdk_api::{ExecutableConfig, RunHook, RunHookResult};
use proto_shim::exec_command_and_replace;
use starbase::AppResult;
use starbase_utils::fs;
use std::env;
use std::ffi::OsStr;
use std::process::Command;
use system_env::create_process_command;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct RunArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

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

fn is_trying_to_self_upgrade(tool: &Tool, args: &[String]) -> bool {
    if tool.metadata.self_upgrade_commands.is_empty() {
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

async fn get_executable(tool: &Tool, args: &RunArgs) -> miette::Result<ExecutableConfig> {
    let tool_dir = tool.get_product_dir();

    // Run an alternate executable (via shim)
    if let Some(alt_name) = &args.alt {
        for location in tool.resolve_shim_locations().await? {
            if &location.name == alt_name {
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

fn create_command<I: IntoIterator<Item = A>, A: AsRef<OsStr>>(
    tool: &Tool,
    exe_config: &ExecutableConfig,
    args: I,
) -> miette::Result<Command> {
    let exe_path = exe_config.exe_path.as_ref().unwrap();
    let args = args
        .into_iter()
        .map(|arg| arg.as_ref().to_os_string())
        .collect::<Vec<_>>();

    let command = if let Some(parent_exe_path) = &exe_config.parent_exe_name {
        let mut exe_args = vec![exe_path.as_os_str().to_os_string()];
        exe_args.extend(args);

        debug!(
            bin = ?parent_exe_path,
            args = ?exe_args,
            pid = std::process::id(),
            "Running {}", tool.get_name(),
        );

        create_process_command(parent_exe_path, exe_args)
    } else {
        debug!(
            bin = ?exe_path,
            args = ?args,
            pid = std::process::id(),
            "Running {}", tool.get_name(),
        );

        create_process_command(exe_path, args)
    };

    Ok(command)
}

#[tracing::instrument(skip_all)]
pub async fn run(session: ProtoSession, args: RunArgs) -> AppResult {
    let mut tool = session.load_tool(&args.id).await?;

    // Avoid running the tool's native self-upgrade as it conflicts with proto
    if is_trying_to_self_upgrade(&tool, &args.passthrough) {
        return Err(ProtoCliError::RunNoSelfUpgrade {
            command: format!("proto install {} --pin", tool.id),
            tool: tool.get_name().to_owned(),
        }
        .into());
    }

    let spec = detect_version_with_spec(&tool, args.spec.clone()).await?;

    // Check if installed or need to install
    if !tool.is_setup_with_spec(&spec).await? {
        let config = tool.proto.load_config()?;
        let resolved_version = tool.get_resolved_version();

        if !config.settings.auto_install {
            let command = format!("proto install {} {}", tool.id, resolved_version);

            if let Ok(source) = env::var(format!("{}_DETECTED_FROM", tool.get_env_var_prefix())) {
                return Err(ProtoCliError::RunMissingToolWithSource {
                    tool: tool.get_name().to_owned(),
                    version: spec.to_string(),
                    command,
                    path: source.into(),
                }
                .into());
            }

            return Err(ProtoCliError::RunMissingTool {
                tool: tool.get_name().to_owned(),
                version: spec.to_string(),
                command,
            }
            .into());
        }

        // Install the tool
        session.console.out.write_line(format!(
            "Auto-install is enabled, attempting to install {} {}",
            tool.get_name(),
            resolved_version,
        ))?;

        install_one(
            session.clone(),
            InstallArgs {
                internal: true,
                spec: Some(ToolSpec {
                    backend: spec.backend,
                    req: resolved_version.to_unresolved_spec(),
                    res: Some(resolved_version.clone()),
                }),
                ..Default::default()
            },
            tool.id.clone(),
        )
        .await?;

        session.console.out.write_line(format!(
            "{} {} has been installed, continuing execution...",
            tool.get_name(),
            resolved_version,
        ))?;
    }

    // Determine the binary path to execute
    let exe_config = get_executable(&tool, &args).await?;
    let exe_path = exe_config
        .exe_path
        .as_ref()
        .expect("Could not determine executable path.");

    // Run before hook
    let hook_result = if tool.plugin.has_func("pre_run").await {
        let globals_dir = tool.locate_globals_dir().await?;
        let globals_prefix = tool.locate_globals_prefix().await?;

        // Ensure directory exists as some tools require it
        if let Some(dir) = &globals_dir {
            let _ = fs::create_dir_all(dir);
        }

        tool.plugin
            .call_func_with(
                "pre_run",
                RunHook {
                    context: tool.create_context(),
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

    for (key, value) in tool.proto.load_config()?.get_env_vars(Some(&tool.id))? {
        match value {
            Some(value) => {
                command.env(key, value);
            }
            None => {
                command.env_remove(key);
            }
        };
    }

    if let Some(hook_args) = hook_result.args {
        command.args(hook_args);
    }

    if let Some(hook_env) = hook_result.env {
        command.envs(hook_env);
    }

    command
        .env(
            format!("{}_VERSION", tool.get_env_var_prefix()),
            tool.get_resolved_version().to_string(),
        )
        .env(
            format!("{}_BIN", tool.get_env_var_prefix()),
            exe_path.to_string_lossy().to_string(),
        );

    // Update the last used timestamp
    if env::var("PROTO_SKIP_USED_AT").is_err() {
        let _ = tool.product.track_used_at();
    }

    // Must be the last line!
    exec_command_and_replace(command)
        .into_diagnostic()
        .map(|_| None)
}
