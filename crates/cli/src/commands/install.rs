use crate::helpers::{create_progress_bar, disable_progress_bars};
use crate::hooks::go as go_hooks;
use crate::tools::{create_tool, ToolType};
use async_recursion::async_recursion;
use proto_core::color;
use starbase::SystemResult;
use tracing::{debug, info};

#[async_recursion]
pub async fn install(
    tool_type: ToolType,
    version: Option<String>,
    pin_version: bool,
    passthrough: Vec<String>,
) -> SystemResult {
    let version = version.unwrap_or_else(|| "latest".into());
    let mut tool = create_tool(&tool_type).await?;

    if tool.is_setup(&version).await? {
        info!(
            "{} has already been installed at {}",
            tool.get_name(),
            color::path(tool.get_install_dir()?),
        );

        return Ok(());
    }

    // Rust doesn't download files but runs commands
    if matches!(tool_type, ToolType::Rust) {
        disable_progress_bars();
    }

    debug!(
        "Installing {} with version \"{}\"",
        tool.get_name(),
        version,
    );

    let pb = create_progress_bar(format!(
        "Installing {} v{}",
        tool.get_name(),
        tool.get_resolved_version()
    ));

    tool.setup(&version).await?;
    tool.cleanup().await?;

    if pin_version {
        let version = tool.get_resolved_version().to_owned();
        let manifest = tool.get_manifest_mut()?;
        manifest.default_version = Some(version);
        manifest.save()?;
    }

    pb.finish_and_clear();

    info!(
        "{} has been installed at {}!",
        tool.get_name(),
        color::path(tool.get_install_dir()?),
    );

    // Support post install actions that are not coupled to the
    // `Tool` trait. Right now we are hard-coding this, but we
    // should provide a better API.

    if let ToolType::Plugin(id) = tool_type {
        if id == "go" {
            go_hooks::post_install(&passthrough)?;
        }

        if id == "node" && !passthrough.contains(&"--no-bundled-npm".to_string()) {
            info!("Installing npm that comes bundled with {}", tool.get_name());

            // This ensures that the correct version is used by the npm tool
            std::env::set_var("PROTO_NODE_VERSION", tool.get_resolved_version());

            install(
                ToolType::Plugin("npm".into()),
                Some("bundled".into()),
                pin_version,
                passthrough,
            )
            .await?;
        }
    }

    Ok(())
}
