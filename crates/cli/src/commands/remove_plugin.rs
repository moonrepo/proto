use crate::commands::tool;
use starbase::system;
use starbase_styles::color;
use tracing::warn;

#[system]
pub async fn remove_plugin_old() {
    warn!(
        "This command is deprecated, use {} instead",
        color::shell("proto tool remove")
    );

    tool::remove_tool(states, resources, emitters).await?;
}
