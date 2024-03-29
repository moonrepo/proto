use crate::error::ProtoCliError;
use crate::helpers::ProtoResource;
use clap::Args;
use comfy_table::presets::NOTHING;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
use proto_core::{ProtoError, UnresolvedVersionSpec, VersionSpec};
use semver::VersionReq;
use serde::Serialize;
use starbase::system;
use starbase_styles::color::Style;
use starbase_utils::json;
use std::collections::BTreeMap;
use std::path::PathBuf;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct OutdatedArgs {
    #[arg(long, help = "Include versions in global .prototools")]
    include_global: bool,

    #[arg(long, help = "Print the list in JSON format")]
    json: bool,

    #[arg(
        long,
        help = "Check for latest available version ignoring requirements and ranges"
    )]
    latest: bool,

    #[arg(long, help = "Only check versions in local .prototools")]
    only_local: bool,

    #[arg(long, help = "Update and write the versions to the local .prototools")]
    update: bool,
}

#[derive(Serialize)]
pub struct OutdatedItem {
    is_latest: bool,
    is_outdated: bool,
    config_source: PathBuf,
    config_version: UnresolvedVersionSpec,
    current_version: VersionSpec,
    newest_version: VersionSpec,
    latest_version: VersionSpec,
}

fn get_in_major_range(spec: &UnresolvedVersionSpec) -> UnresolvedVersionSpec {
    match spec {
        UnresolvedVersionSpec::Version(version) => UnresolvedVersionSpec::Req(
            VersionReq::parse(format!("~{}", version.major).as_str()).unwrap(),
        ),
        _ => spec.clone(),
    }
}

#[system]
pub async fn outdated(args: ArgsRef<OutdatedArgs>, proto: ResourceRef<ProtoResource>) {
    let manager = proto.env.load_config_manager()?;

    debug!("Checking for newer versions...");

    let mut items = BTreeMap::default();
    let initial_version = UnresolvedVersionSpec::default(); // latest

    for file in &manager.files {
        if !file.exists
            || !args.include_global && file.global
            || args.only_local && file.path != proto.env.cwd
        {
            continue;
        }

        let Some(file_versions) = &file.config.versions else {
            continue;
        };

        for (tool_id, config_version) in file_versions {
            if items.contains_key(tool_id) {
                continue;
            }

            let mut tool = proto.load_tool(tool_id).await?;
            tool.disable_caching();

            debug!("Checking {}", tool.get_name());

            let version_resolver = tool.load_version_resolver(&initial_version).await?;

            let handle_error = || ProtoError::VersionResolveFailed {
                tool: tool.get_name().to_owned(),
                version: initial_version.to_string(),
            };

            let current_version = version_resolver
                .resolve(config_version)
                .ok_or_else(handle_error)?;

            let newest_version = version_resolver
                .resolve_without_manifest(&get_in_major_range(config_version))
                .ok_or_else(handle_error)?;

            let latest_version = version_resolver
                .resolve_without_manifest(&initial_version)
                .ok_or_else(handle_error)?;

            items.insert(
                tool.id,
                OutdatedItem {
                    is_latest: current_version == latest_version,
                    is_outdated: newest_version > current_version
                        || latest_version > current_version,
                    config_source: file.path.to_owned(),
                    config_version: config_version.to_owned(),
                    current_version,
                    newest_version,
                    latest_version,
                },
            );
        }
    }

    if items.is_empty() {
        return Err(ProtoCliError::NoConfiguredTools.into());
    }

    // if args.update {
    //     ProtoConfig::update(&proto.env.cwd, |config| {
    //         config
    //             .versions
    //             .get_or_insert(Default::default())
    //             .extend(tool_versions);
    //     })?;
    // }

    if args.json {
        println!("{}", json::format(&items, true)?);

        return Ok(());
    }

    let mut table = Table::new();
    table.load_preset(NOTHING);
    table.set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(vec![
        Cell::new("Tool").add_attribute(Attribute::Bold),
        Cell::new("Current").add_attribute(Attribute::Bold),
        Cell::new("Newest").add_attribute(Attribute::Bold),
        Cell::new("Latest").add_attribute(Attribute::Bold),
        Cell::new("Config").add_attribute(Attribute::Bold),
    ]);

    for (id, item) in items {
        table.add_row(vec![
            Cell::new(&id).fg(Color::AnsiValue(Style::Id.color() as u8)),
            Cell::new(&item.current_version),
            if item.newest_version == item.current_version {
                Cell::new(&item.newest_version)
                    .fg(Color::AnsiValue(Style::MutedLight.color() as u8))
            } else {
                Cell::new(&item.newest_version).fg(Color::AnsiValue(Style::Success.color() as u8))
            },
            if item.latest_version == item.current_version {
                Cell::new(&item.latest_version)
                    .fg(Color::AnsiValue(Style::MutedLight.color() as u8))
            } else if item.latest_version == item.newest_version {
                Cell::new(&item.latest_version).fg(Color::AnsiValue(Style::Success.color() as u8))
            } else {
                Cell::new(&item.latest_version).fg(Color::AnsiValue(Style::Failure.color() as u8))
            },
            Cell::new(item.config_source.to_string_lossy())
                .fg(Color::AnsiValue(Style::Path.color() as u8)),
        ]);
    }

    println!("\n{table}\n");
}
