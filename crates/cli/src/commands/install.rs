use crate::helpers::enable_logging;
use crate::manifest::Manifest;
use crate::tools::{create_tool, ToolType};
use log::info;
use proto_core::{color, ProtoError};

pub async fn install(tool_type: ToolType, version: Option<String>) -> Result<(), ProtoError> {
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

    let version = tool.get_resolved_version();
    let mut manifest = Manifest::load_for_tool(&tool)?;

    if manifest.default_version.is_none() {
        manifest.default_version = Some(version.to_owned());
    }

    manifest.installed_versions.insert(version.to_owned());
    manifest.save()?;

    info!(
        target: "proto:install", "{} has been installed at {}!",
        tool.get_name(),
        color::path(tool.get_install_dir()?),
    );

    Ok(())
}
