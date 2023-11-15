use crate::commands::tool;
use starbase::system;
use starbase_styles::color;
use tracing::warn;

#[system]
pub async fn tools() {
    warn!(
        "This command is deprecated, use {} instead",
        color::shell("proto tool list")
    );

    tool::list(states, resources, emitters).await?;
}
