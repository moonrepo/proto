use clap::Args;
use proto_core::{load_tool, ToolsConfig, UnresolvedVersionSpec, VersionSpec};
use starbase::system;
use starbase_styles::color::{self, OwoStyle};
use std::process;
use tracing::{debug, info};

#[derive(Args, Clone, Debug)]
pub struct OutdatedArgs {
    #[arg(
        long,
        help = "Check for latest version available ignoring requirements and ranges"
    )]
    latest: bool,
}

#[system]
pub async fn outdated(args: ArgsRef<OutdatedArgs>) {
    let tools_config = ToolsConfig::load_upwards()?;
    let initial_version = UnresolvedVersionSpec::default(); // latest

    if tools_config.tools.is_empty() {
        eprintln!("No configured tools in .prototools");
        process::exit(1);
    }

    info!("Checking for latest versions");

    for (tool_id, config_version) in &tools_config.tools {
        let mut tool = load_tool(tool_id).await?;
        tool.disable_caching();

        debug!("Checking {}", tool.get_name());

        let mut comments = vec![];
        let versions = tool.load_version_resolver(&initial_version).await?;
        let current_version = versions.resolve(config_version)?;

        comments.push(format!(
            "{} {}",
            color::muted_light("current version"),
            color::symbol(current_version.to_string())
        ));

        let latest_version = versions.resolve_without_manifest(if args.latest {
            &initial_version // latest alias
        } else {
            config_version // req, range, etc
        })?;

        comments.push(format!(
            "{} {}",
            color::muted_light(if args.latest {
                "latest version"
            } else {
                "updatable version"
            }),
            color::symbol(latest_version.to_string())
        ));

        let is_outdated = match (current_version, latest_version) {
            (VersionSpec::Version(a), VersionSpec::Version(b)) => b > a,
            _ => false,
        };

        if is_outdated {
            comments.push(color::success("update available!"));
        }

        println!(
            "{} {} {}",
            OwoStyle::new().bold().style(color::id(&tool.id)),
            color::muted("-"),
            comments.join(&color::muted(" - "))
        );
    }
}
