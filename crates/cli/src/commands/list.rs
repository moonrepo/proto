use crate::helpers::ProtoResource;
use clap::Args;
use proto_core::Id;
use starbase::system;
use std::process;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct ListArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(long, help = "Include local aliases in the output")]
    aliases: bool,
}

#[system]
pub async fn list(args: ArgsRef<ListArgs>, proto: ResourceRef<ProtoResource>) {
    let tool = proto.load_tool(&args.id).await?;

    debug!(manifest = ?tool.manifest.path, "Using versions from manifest");

    let mut versions = Vec::from_iter(tool.manifest.installed_versions);

    if versions.is_empty() {
        eprintln!("No versions installed");
        process::exit(1);
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
        let config = proto.env.load_config()?;

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
}
