use crate::error::ProtoCliError;
use clap::Args;
use miette::IntoDiagnostic;
use proto_core::{load_tool_with_proto, ProtoEnvironment, UnresolvedVersionSpec, VersionSpec};
use serde::Serialize;
use starbase::system;
use starbase_styles::color::{self, OwoStyle};
use starbase_utils::json;
use std::collections::HashMap;
use tracing::{debug, info};

#[derive(Args, Clone, Debug)]
pub struct OutdatedArgs {
    #[arg(long, help = "Print the list in JSON format")]
    json: bool,

    #[arg(
        long,
        help = "Check for latest available version ignoring requirements and ranges"
    )]
    latest: bool,

    #[arg(long, help = "Update and write the versions to the local .prototools")]
    update: bool,
}

#[derive(Serialize)]
pub struct OutdatedItem {
    is_latest: bool,
    version_config: UnresolvedVersionSpec,
    current_version: VersionSpec,
    newer_version: VersionSpec,
}

#[system]
pub async fn outdated(args: ArgsRef<OutdatedArgs>) {
    let proto = ProtoEnvironment::new()?;
    let config = proto.load_config()?;

    if config.versions.is_empty() {
        return Err(ProtoCliError::NoConfiguredTools.into());
    }

    if !args.json {
        info!("Checking for newer versions...");
    }

    let mut items = HashMap::new();
    let mut tool_versions = HashMap::new();
    let initial_version = UnresolvedVersionSpec::default(); // latest

    for (tool_id, config_version) in &config.versions {
        let mut tool = load_tool_with_proto(tool_id, &proto).await?;
        tool.disable_caching();

        debug!("Checking {}", tool.get_name());

        let mut comments = vec![];
        let versions = tool.load_version_resolver(&initial_version).await?;
        let current_version = versions.resolve(config_version)?;
        let is_latest = args.latest || matches!(config_version, UnresolvedVersionSpec::Version(_));

        comments.push(format!(
            "current version {} {}",
            color::symbol(current_version.to_string()),
            color::muted_light(format!("(via {})", config_version))
        ));

        let newer_version = versions.resolve_without_manifest(if is_latest {
            &initial_version // latest alias
        } else {
            config_version // req, range, etc
        })?;

        comments.push(format!(
            "{} {}",
            if is_latest {
                "latest version"
            } else {
                "newer version"
            },
            color::symbol(newer_version.to_string())
        ));

        let is_outdated = match (&current_version, &newer_version) {
            (VersionSpec::Version(a), VersionSpec::Version(b)) => b > a,
            _ => false,
        };

        if is_outdated {
            comments.push(color::success("update available!"));
        }

        if args.update {
            tool_versions.insert(tool.id.clone(), newer_version.to_unresolved_spec());
        }

        if args.json {
            items.insert(
                tool.id,
                OutdatedItem {
                    is_latest,
                    version_config: config_version.to_owned(),
                    current_version,
                    newer_version,
                },
            );
        } else {
            println!(
                "{} {} {}",
                OwoStyle::new().bold().style(color::id(&tool.id)),
                color::muted("-"),
                comments.join(&color::muted_light(", "))
            );
        }
    }

    if args.update {
        // TODO update
        // tools_config.tools.extend(tool_versions);
        // tools_config.save()?;
    }

    if args.json {
        println!("{}", json::to_string_pretty(&items).into_diagnostic()?);
    }
}
