use crate::helpers::{create_progress_bar, disable_progress_bars};
use crate::hooks::go as go_hooks;
use crate::tools::create_tool;
use async_recursion::async_recursion;
use starbase::SystemResult;
use starbase_styles::color;
use tracing::{debug, info};

#[async_recursion]
pub async fn install(
    tool_id: String,
    version: Option<String>,
    pin_version: bool,
    passthrough: Vec<String>,
) -> SystemResult {
    let version = version.unwrap_or_else(|| "latest".into());
    let mut tool = create_tool(&tool_id).await?;

    if tool.is_setup(&version).await? {
        info!(
            "{} has already been installed at {}",
            tool.get_name(),
            color::path(tool.get_tool_dir()),
        );

        return Ok(());
    }

    // Rust doesn't download files but runs commands
    // TODO move to plugin?
    if tool_id == "rust" {
        disable_progress_bars();
    }

    debug!(
        "Installing {} with version \"{}\"",
        tool.get_name(),
        version,
    );

    let pb = create_progress_bar(format!(
        "Installing {} {}",
        tool.get_name(),
        tool.get_resolved_version()
    ));

    tool.setup(&version).await?;
    // TODO
    // tool.cleanup().await?;

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

    // Support post install actions that are not coupled to the
    // `Tool` trait. Right now we are hard-coding this, but we
    // should provide a better API.

    if tool_id == "go" {
        go_hooks::post_install(&passthrough)?;
    }

    if tool_id == "node" && !passthrough.contains(&"--no-bundled-npm".to_string()) {
        info!("Installing npm that comes bundled with {}", tool.get_name());

        // This ensures that the correct version is used by the npm tool
        std::env::set_var(
            "PROTO_NODE_VERSION",
            tool.get_resolved_version().to_string(),
        );

        install(
            "npm".into(),
            Some("bundled".into()),
            pin_version,
            passthrough,
        )
        .await?;
    }

    Ok(())
}
