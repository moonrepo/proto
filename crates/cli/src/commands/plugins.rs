use crate::commands::tool;
use starbase::system;
use starbase_styles::color;
use tracing::warn;

#[system]
pub async fn plugins() {
    warn!(
        "This command is deprecated, use {} instead",
        color::shell("proto tool list-plugins")
    );

    tool::list_plugins(states, resources, emitters).await?;
}
