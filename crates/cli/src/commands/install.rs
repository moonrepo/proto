use crate::helpers::{create_progress_bar, disable_progress_bars};
use crate::shell;
use crate::tools::create_tool;
use miette::IntoDiagnostic;
use proto_core::{Id, Tool, VersionType};
use proto_pdk_api::{InstallHook, SyncShellProfileInput, SyncShellProfileOutput};
use starbase::SystemResult;
use starbase_styles::color;
use std::env;
use tracing::{debug, info};

pub async fn install(
    tool_id: Id,
    version: Option<VersionType>,
    pin_version: bool,
    passthrough: Vec<String>,
) -> SystemResult {
    let version = version.unwrap_or_default();
    let mut tool = create_tool(&tool_id).await?;

    if tool.is_setup(&version).await? {
        info!(
            "{} has already been installed at {}",
            tool.get_name(),
            color::path(tool.get_tool_dir()),
        );

        return Ok(());
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

    env::set_var("PROTO_INSTALL", tool_id.to_string());

    // Run before hook
    tool.run_hook(
        "pre_install",
        InstallHook {
            passthrough_args: passthrough.clone(),
            pinned: pin_version,
            resolved_version: resolved_version.to_string(),
        },
    )?;

    // Install the tool
    debug!("Installing {} with version {}", tool.get_name(), version);

    let pb = create_progress_bar(format!(
        "Installing {} {}",
        tool.get_name(),
        tool.get_resolved_version()
    ));

    tool.setup(&version).await?;
    tool.cleanup().await?;

    if pin_version {
        tool.manifest.default_version = Some(tool.get_resolved_version());
        tool.manifest.save()?;
    }

    pb.finish_and_clear();

    info!(
        "{} has been installed to {}!",
        tool.get_name(),
        color::path(tool.get_tool_dir()),
    );

    // Run after hook
    tool.run_hook(
        "post_install",
        InstallHook {
            passthrough_args: passthrough.clone(),
            pinned: pin_version,
            resolved_version: resolved_version.to_string(),
        },
    )?;

    // Sync shell profile
    update_shell(tool, passthrough)?;

    Ok(())
}

fn update_shell(tool: Tool, passthrough_args: Vec<String>) -> miette::Result<()> {
    if !tool.plugin.has_func("sync_shell_profile") {
        return Ok(());
    }

    let output: SyncShellProfileOutput = tool.plugin.call_func_with(
        "sync_shell_profile",
        SyncShellProfileInput {
            env: tool.create_environment()?,
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
