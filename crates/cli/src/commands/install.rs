use super::clean::clean_plugins;
use super::pin::{internal_pin, PinArgs};
use crate::helpers::{create_progress_bar, disable_progress_bars};
use crate::shell;
use clap::Args;
use miette::IntoDiagnostic;
use proto_core::{load_tool, Id, PinType, Tool, UnresolvedVersionSpec};
use proto_pdk_api::{InstallHook, SyncShellProfileInput, SyncShellProfileOutput};
use starbase::{system, SystemResult};
use starbase_styles::color;
use std::env;
use tracing::{debug, info};

#[derive(Args, Clone, Debug)]
pub struct InstallArgs {
    #[arg(required = true, help = "ID of tool")]
    pub id: Id,

    #[arg(
        default_value = "latest",
        help = "Version or alias of tool",
        group = "version-type"
    )]
    pub spec: Option<UnresolvedVersionSpec>,

    #[arg(
        long,
        help = "Install a canary (nightly, etc) version",
        group = "version-type"
    )]
    pub canary: bool,

    #[arg(
        long,
        help = "Pin version as the global default and create a binary symlink"
    )]
    pub pin: bool,

    // Passthrough args (after --)
    #[arg(last = true, help = "Unique arguments to pass to each tool")]
    pub passthrough: Vec<String>,
}

async fn pin_version(
    tool: &mut Tool,
    initial_version: &UnresolvedVersionSpec,
    global: bool,
) -> SystemResult {
    let mut args = PinArgs {
        id: tool.id.clone(),
        spec: tool.get_resolved_version().to_unresolved_spec(),
        global: false,
    };

    // via `--pin` arg
    if global {
        args.global = true;

        return internal_pin(tool, &args, true).await;
    }

    // via `pin-latest` setting
    if initial_version.is_latest() {
        let user_config = tool.proto.load_user_config()?;

        if let Some(pin_type) = &user_config.pin_latest {
            args.global = matches!(pin_type, PinType::Global);

            return internal_pin(tool, &args, true).await;
        }
    }

    Ok(())
}

pub async fn internal_install(args: InstallArgs, tool: Option<Tool>) -> miette::Result<Tool> {
    let mut tool = match tool {
        Some(tool) => tool,
        None => load_tool(&args.id).await?,
    };

    let version = if args.canary {
        UnresolvedVersionSpec::Canary
    } else {
        args.spec.clone().unwrap_or_default()
    };

    // Disable version caching and always use the latest when installing
    tool.disable_caching();

    if !version.is_canary() && tool.is_setup(&version).await? {
        pin_version(&mut tool, &version, args.pin).await?;

        info!(
            "{} has already been installed at {}",
            tool.get_name(),
            color::path(tool.get_tool_dir()),
        );

        return Ok(tool);
    }

    if tool.disable_progress_bars() {
        disable_progress_bars();
    }

    let resolved_version = tool.get_resolved_version();

    // This ensures that the correct version is used by other processes
    env::set_var(
        format!("{}_VERSION", tool.get_env_var_prefix()),
        resolved_version.to_string(),
    );

    env::set_var("PROTO_INSTALL", args.id.to_string());

    // Run before hook
    tool.run_hook("pre_install", || InstallHook {
        context: tool.create_context(),
        passthrough_args: args.passthrough.clone(),
        pinned: args.pin,
    })?;

    // Install the tool
    debug!("Installing {} with version {}", tool.get_name(), version);

    let pb = create_progress_bar(format!(
        "Installing {} {}",
        tool.get_name(),
        tool.get_resolved_version()
    ));

    let installed = tool.setup(&version, false).await?;

    pb.finish_and_clear();

    if !installed {
        return Ok(tool);
    }

    pin_version(&mut tool, &version, args.pin).await?;

    info!(
        "{} has been installed to {}!",
        tool.get_name(),
        color::path(tool.get_tool_dir()),
    );

    // Run after hook
    tool.run_hook("post_install", || InstallHook {
        context: tool.create_context(),
        passthrough_args: args.passthrough.clone(),
        pinned: args.pin,
    })?;

    // Sync shell profile
    update_shell(&tool, args.passthrough.clone())?;

    // Clean plugins
    debug!("Auto-cleaning plugins");

    clean_plugins(7).await?;

    Ok(tool)
}

fn update_shell(tool: &Tool, passthrough_args: Vec<String>) -> miette::Result<()> {
    if !tool.plugin.has_func("sync_shell_profile") {
        return Ok(());
    }

    let output: SyncShellProfileOutput = tool.plugin.call_func_with(
        "sync_shell_profile",
        SyncShellProfileInput {
            context: tool.create_context(),
            passthrough_args,
        },
    )?;

    if output.skip_sync {
        return Ok(());
    }

    let shell_type = shell::detect_shell(None);
    let mut env_vars = vec![];

    if let Some(export_vars) = output.export_vars {
        env_vars.extend(export_vars);
    }

    if let Some(extend_path) = output.extend_path {
        env_vars.push((
            "PATH".to_string(),
            env::join_paths(extend_path)
                .into_diagnostic()?
                .to_string_lossy()
                .to_string(),
        ));
    }

    debug!(shell = ?shell_type, env_vars = ?env_vars, "Updating shell profile");

    if let Some(content) = shell::format_env_vars(&shell_type, tool.id.as_str(), env_vars) {
        if let Some(updated_profile) =
            shell::write_profile_if_not_setup(&shell_type, content, &output.check_var)?
        {
            info!(
                "Added {} to shell profile {}",
                output.check_var,
                color::path(updated_profile)
            );
        }
    }

    Ok(())
}

#[system]
pub async fn install(args: ArgsRef<InstallArgs>) {
    internal_install(args.to_owned(), None).await?;
}
