use crate::helpers::enable_logging;
use log::info;
use proto::{color, create_tool, ProtoError, ToolType};

pub async fn install(tool_type: ToolType, version: Option<String>) -> Result<(), ProtoError> {
    enable_logging();

    let version = version.unwrap_or_else(|| "latest".into());
    let mut tool = create_tool(&tool_type)?;

    if tool.is_setup(&version).await? {
        info!(
            target: "proto:install",
            "{} has already been installed to {}",
            tool.get_name(),
            color::path(tool.get_install_dir()?),
        );
    } else {
        info!(
            target: "proto:install",
            "Installing {} with version \"{}\"",
            tool.get_name(),
            version,
        );

        tool.setup(&version).await?;

        info!(
            target: "proto:install", "{} has been installed to {}!",
            tool.get_name(),
            color::path(tool.get_install_dir()?),
        );
    }

    Ok(())
}
