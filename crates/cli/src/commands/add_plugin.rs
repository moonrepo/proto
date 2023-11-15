use crate::commands::plugin;
use starbase::system;
use starbase_styles::color;
use tracing::warn;

#[system]
pub async fn add_plugin_old() {
    warn!(
        "This command is deprecated, use {} instead",
        color::shell("proto plugin add")
    );

    plugin::add_plugin(states, resources, emitters).await?;
}
