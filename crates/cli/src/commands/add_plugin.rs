use crate::commands::tool;
use starbase::system;
use starbase_styles::color;
use tracing::warn;

#[system]
pub async fn add_plugin_old() {
    warn!(
        "This command is deprecated, use {} instead",
        color::shell("proto tool add")
    );

    tool::add_tool(states, resources, emitters).await?;
}
