use crate::commands::install::{internal_install, InstallArgs};
use crate::error::ProtoCliError;
use crate::session::ProtoSession;
use clap::Args;
use indexmap::IndexMap;
use miette::IntoDiagnostic;
use proto_core::{detect_version, Id, ProtoError, Tool, UnresolvedVersionSpec, ENV_VAR_SUB};
use proto_pdk_api::{ExecutableConfig, RunHook, RunHookResult};
use proto_shim::exec_command_and_replace;
use starbase::AppResult;
use std::env;
use std::ffi::OsStr;
use std::process::Command;
use system_env::create_process_command;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct RunArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(help = "Version or alias of tool")]
    spec: Option<UnresolvedVersionSpec>,

    #[arg(long, help = "Name of an alternate (secondary) binary to run")]
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

    for arg in args {
        // Find first non-option arg
        if arg.starts_with('-') {
            continue;
        }

        // And then check if an upgrade command
        return tool.metadata.self_upgrade_commands.contains(arg);
    }

    false
}

fn get_executable(tool: &Tool, args: &RunArgs) -> miette::Result<ExecutableConfig> {
    let tool_dir = tool.get_product_dir();

    // Run an alternate executable (via shim)
    if let Some(alt_name) = &args.alt {
        for location in tool.get_shim_locations()? {
            if location.name == *alt_name {
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

        return Err(ProtoCliError::MissingRunAltBin {
            bin: alt_name.to_owned(),
            path: tool_dir,
        }
        .into());
    }

    // Otherwise use the primary
    let mut config = tool
        .get_exe_location()?
        .expect("Required executable information missing!")
        .config;

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

// We don't use a `BTreeMap` for env vars, so that variable interpolation
// and order of declaration can work correctly!
fn get_env_vars(tool: &Tool) -> miette::Result<IndexMap<&str, Option<String>>> {
    let config = tool.proto.load_config()?;
    let mut base_vars = IndexMap::new();

    base_vars.extend(config.env.iter());

    if let Some(tool_config) = config.tools.get(&tool.id) {
        base_vars.extend(tool_config.env.iter())
    }

    let mut vars = IndexMap::<&str, Option<String>>::new();

    for (key, value) in base_vars {
        let key_exists = env::var(key).is_ok_and(|v| !v.is_empty());
        let value = value.to_value();

        // Don't override parent inherited vars
        if key_exists && value.is_some() {
            continue;
        }

        // Interpolate nested vars
        let value = value.map(|val| {
            ENV_VAR_SUB
                .replace_all(&val, |cap: &regex::Captures| {
                    let name = cap.get(1).unwrap().as_str();

                    if let Ok(existing) = env::var(name) {
                        existing
                    } else if let Some(Some(existing)) = vars.get(name) {
                        existing.to_owned()
                    } else {
                        String::new()
                    }
                })
                .to_string()
        });

        vars.insert(key.as_str(), value);
    }

    Ok(vars)
}

pub async fn run(session: ProtoSession, args: RunArgs) -> AppResult {
    let mut tool = session.load_tool(&args.id).await?;

    // Avoid running the tool's native self-upgrade as it conflicts with proto
    if is_trying_to_self_upgrade(&tool, &args.passthrough) {
        return Err(ProtoCliError::NoSelfUpgrade {
            command: format!("proto install {} --pin", tool.id),
            tool: tool.get_name().to_owned(),
        }
        .into());
    }

    let version = detect_version(&tool, args.spec.clone()).await?;

    // Check if installed or install
    if !tool.is_setup(&version).await? {
        let config = tool.proto.load_config()?;

        if !config.settings.auto_install {
            let command = format!("proto install {} {}", tool.id, tool.get_resolved_version());

            if let Ok(source) = env::var("PROTO_DETECTED_FROM") {
                return Err(ProtoError::MissingToolForRunWithSource {
                    tool: tool.get_name().to_owned(),
                    version: version.to_string(),
                    command,
                    path: source.into(),
                }
                .into());
            }

            return Err(ProtoError::MissingToolForRun {
                tool: tool.get_name().to_owned(),
                version: version.to_string(),
                command,
            }
            .into());
        }

        // Install the tool
        debug!("Auto-install setting is configured, attempting to install");

        tool = internal_install(
            &session,
            InstallArgs {
                canary: false,
                id: args.id.clone(),
                pin: None,
                passthrough: vec![],
                spec: Some(tool.get_resolved_version().to_unresolved_spec()),
            },
            Some(tool),
        )
        .await?;
    }

    // Determine the binary path to execute
    let exe_config = get_executable(&tool, &args)?;
    let exe_path = exe_config
        .exe_path
        .as_ref()
        .expect("Could not determine executable path.");

    // Run before hook
    let hook_result = if tool.plugin.has_func("pre_run") {
        tool.locate_globals_dir().await?;

        let globals_dir = tool.get_globals_bin_dir();
        let globals_prefix = tool.get_globals_prefix();

        tool.plugin.call_func_with(
            "pre_run",
            RunHook {
                context: tool.create_context(),
                globals_dir: globals_dir.map(|dir| tool.to_virtual_path(dir)),
                globals_prefix: globals_prefix.map(|p| p.to_owned()),
                passthrough_args: args.passthrough.clone(),
            },
        )?
    } else {
        RunHookResult::default()
    };

    // Create and run the command
    let mut command = create_command(&tool, &exe_config, &args.passthrough)?;

    for (key, value) in get_env_vars(&tool)? {
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
    exec_command_and_replace(command).into_diagnostic()
}
