use clap::Args;
use proto_core::{load_tool, Id, UserConfig};
use starbase::system;
use starbase_styles::color;
use tracing::info;

#[derive(Args, Clone, Debug)]
pub struct UnaliasArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(required = true, help = "Alias name")]
    alias: String,
}

#[system]
pub async fn unalias(args: ArgsRef<UnaliasArgs>) {
    let tool = load_tool(&args.id).await?;
    let mut user_config = UserConfig::load()?;

    let tool_config = user_config.tools.entry(args.id.clone()).or_default();
    let value = tool_config.aliases.remove(&args.alias);

    user_config.save()?;

    if let Some(version) = value {
        info!(
            "Removed alias {} ({}) from {}",
            color::id(&args.alias),
            color::muted_light(version.to_string()),
            tool.get_name(),
        );
    } else {
        info!(
            "Alias {} not found for {}",
            color::id(&args.alias),
            tool.get_name(),
        );
    }
}
