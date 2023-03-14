use crate::helpers::enable_logging;
use crate::shell;
use crate::tools::{create_tool, ToolType};
use async_recursion::async_recursion;
use clap_complete::Shell;
use log::{debug, info};
use proto_core::{color, Manifest, ProtoError};
use rustc_hash::FxHashMap;

#[async_recursion]
pub async fn install(
    tool_type: ToolType,
    version: Option<String>,
    pin_version: bool,
    passthrough: Vec<String>,
) -> Result<(), ProtoError> {
    enable_logging();

    let version = version.unwrap_or_else(|| "latest".into());
    let mut tool = create_tool(&tool_type)?;

    if tool.is_setup(&version).await? {
        info!(
            target: "proto:install",
            "{} has already been installed at {}",
            tool.get_name(),
            color::path(tool.get_install_dir()?),
        );

        return Ok(());
    }

    info!(
        target: "proto:install",
        "Installing {} with version \"{}\"",
        tool.get_name(),
        version,
    );

    tool.setup(&version).await?;

    if pin_version {
        let mut manifest = Manifest::load_for_tool(tool.get_bin_name())?;
        manifest.default_version = Some(tool.get_resolved_version().to_owned());
        manifest.save()?;
    }

    info!(
        target: "proto:install", "{} has been installed at {}!",
        tool.get_name(),
        color::path(tool.get_install_dir()?),
    );

    // Support post install actions that are not coupled to the
    // `Tool` trait. Right now we are hard-coding this, but we
    // should provide a better API.
    match tool_type {
        ToolType::Go => {
            if !passthrough.contains(&"--no-gobin".to_string()) {
                if let Some(shell) = Shell::from_env() {
                    let env_vars = FxHashMap::from_iter([
                        ("GOBIN".to_string(), "$HOME/go/bin".to_string()),
                        ("PATH".to_string(), "$GOBIN".to_string()),
                    ]);

                    if let Some(content) = shell::format_env_vars(&shell, "go", env_vars) {
                        if let Some(updated_profile) =
                            shell::write_profile_if_not_setup(&shell, content, "GOBIN")?
                        {
                            info!(
                                target: "proto:install", "Added GOBIN to your shell profile {}",
                                color::path(updated_profile)
                            );
                        }
                    }
                }
            }
        }
        ToolType::Node => {
            if !passthrough.contains(&"--no-bundled-npm".to_string()) {
                debug!(
                    target: "proto:install", "Installing npm that comes bundled with {}",
                    tool.get_name(),
                );

                // This ensures that the correct version is used by the npm tool
                std::env::set_var("PROTO_NODE_VERSION", tool.get_resolved_version());

                install(
                    ToolType::Npm,
                    Some("bundled".into()),
                    pin_version,
                    passthrough,
                )
                .await?;
            }
        }
        _ => {}
    }

    Ok(())
}
