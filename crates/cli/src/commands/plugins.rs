use crate::commands::plugin;
use starbase::system;
use starbase_styles::color;
use tracing::warn;

#[system]
pub async fn plugins() {
    warn!(
        "This command is deprecated, use {} instead",
        color::shell("proto plugin list")
    );

    plugin::list_plugins(states, resources, emitters).await?;
}
