use crate::session::ProtoSession;
use clap::Args;
use proto_core::Id;
use starbase::AppResult;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct ListArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(long, help = "Include local aliases in the output")]
    aliases: bool,
}

#[tracing::instrument(skip_all)]
pub async fn list(session: ProtoSession, args: ListArgs) -> AppResult {
    let tool = session.load_tool(&args.id).await?;

    debug!(manifest = ?tool.inventory.manifest.path, "Using versions from manifest");

    let mut versions = Vec::from_iter(&tool.inventory.manifest.installed_versions);

    if versions.is_empty() {
        eprintln!("No versions installed");

        return Ok(Some(1));
    }

    versions.sort();

    println!(
        "{}",
        versions
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );

    if args.aliases {
        let config = session.env.load_config()?;

        if let Some(tool_config) = config.tools.get(&tool.id) {
            if !tool_config.aliases.is_empty() {
                println!(
                    "{}",
                    tool_config
                        .aliases
                        .iter()
                        .map(|(k, v)| format!("{k} -> {v}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                );
            }
        }
    }

    Ok(None)
}
