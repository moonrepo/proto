use crate::commands::plugin;
use starbase::system;
use starbase_styles::color;
use tracing::warn;

#[system]
pub async fn remove_plugin_old() {
    warn!(
        "This command is deprecated, use {} instead",
        color::shell("proto plugin remove")
    );

    plugin::remove_plugin(states, resources, emitters).await?;
}
