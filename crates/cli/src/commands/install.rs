use crate::helpers::{create_progress_bar, disable_progress_bars};
use crate::tools::create_tool;
use proto_core::{Id, VersionType};
use proto_pdk_api::InstallHook;
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
            passthrough_args: passthrough,
            pinned: pin_version,
            resolved_version: resolved_version.to_string(),
        },
    )?;

    Ok(())
}
