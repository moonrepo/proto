use crate::error::ProtoCliError;
use crate::helpers::create_theme;
use crate::session::ProtoSession;
use clap::Args;
use comfy_table::presets::NOTHING;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
use dialoguer::Confirm;
use miette::IntoDiagnostic;
use proto_core::{Id, ProtoConfig, UnresolvedVersionSpec, VersionSpec};
use rustc_hash::FxHashSet;
use semver::VersionReq;
use serde::Serialize;
use starbase::AppResult;
use starbase_styles::color::{self, Style};
use starbase_utils::json;
use std::collections::BTreeMap;
use std::io::{stdout, IsTerminal};
use std::path::PathBuf;
use tokio::spawn;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct OutdatedArgs {
    #[arg(long, help = "Include versions from global ~/.proto/.prototools")]
    include_global: bool,

    #[arg(long, help = "Print the outdated tools in JSON format")]
    json: bool,

    #[arg(
        long,
        help = "When updating versions, use the latest version instead of newest"
    )]
    latest: bool,

    #[arg(long, help = "Only check versions in local ./.prototools")]
    only_local: bool,

    #[arg(
        long,
        help = "Update and write the versions to their respective configuration"
    )]
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

pub async fn outdated(session: ProtoSession, args: OutdatedArgs) -> AppResult {
    let manager = session.env.load_config_manager()?;

    debug!("Determining outdated tools based on config...");

    let mut configured_tools = BTreeMap::default();

    for file in manager.files.iter().rev() {
        if !file.exists
            || !args.include_global && file.global
            || args.only_local && !file.path.parent().is_some_and(|p| p == session.env.cwd)
        {
            continue;
        }

        if let Some(file_versions) = &file.config.versions {
            for (tool_id, config_version) in file_versions {
                configured_tools.insert(
                    tool_id.to_owned(),
                    (config_version.to_owned(), file.path.to_owned()),
                );
            }
        }
    }

    if configured_tools.is_empty() {
        return Err(ProtoCliError::NoConfiguredTools.into());
    }

    debug!(
        tools = ?configured_tools.keys().map(|id| id.as_str()).collect::<Vec<_>>(),
        "Found tools with configured versions, loading them",
    );

    let tools = session
        .load_tools_with_filters(FxHashSet::from_iter(configured_tools.keys()))
        .await?;
    let mut futures = vec![];

    for mut tool in tools {
        let Some((config_version, config_source)) = configured_tools.remove(&tool.id) else {
            continue;
        };

        futures.push(spawn(async move {
            tool.disable_caching();

            debug!("Checking {}", tool.get_name());

            let initial_version = UnresolvedVersionSpec::default(); // latest
            let version_resolver = tool.load_version_resolver(&initial_version).await?;

            debug!(
                id = tool.id.as_str(),
                config = config_version.to_string(),
                "Resolving current version"
            );

            let current_version =
                tool.resolve_version_candidate(&version_resolver, &config_version, true)?;
            let newest_range = get_in_major_range(&config_version);

            debug!(
                id = tool.id.as_str(),
                range = newest_range.to_string(),
                "Resolving newest version"
            );

            let newest_version =
                tool.resolve_version_candidate(&version_resolver, &newest_range, false)?;

            debug!(
                id = tool.id.as_str(),
                alias = initial_version.to_string(),
                "Resolving latest version"
            );

            let latest_version =
                tool.resolve_version_candidate(&version_resolver, &initial_version, true)?;

            Result::<_, miette::Report>::Ok((
                tool.id,
                OutdatedItem {
                    is_latest: current_version == latest_version,
                    is_outdated: newest_version > current_version
                        || latest_version > current_version,
                    config_source,
                    config_version,
                    current_version,
                    newest_version,
                    latest_version,
                },
            ))
        }));
    }

    let mut items = BTreeMap::default();

    for future in futures {
        let (id, item) = future.await.into_diagnostic()??;

        items.insert(id, item);
    }

    // Dump all the data as JSON
    if args.json {
        println!("{}", json::format(&items, true)?);

        return Ok(());
    }

    // Print all the data in a table
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

    for (id, item) in &items {
        table.add_row(vec![
            Cell::new(id).fg(Color::AnsiValue(Style::Id.color() as u8)),
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

    // If updating versions, batch the changes based on config paths
    let theme = create_theme();

    if args.update
        && (!stdout().is_terminal()
            || Confirm::with_theme(&theme)
                .with_prompt(if args.latest {
                    "Update config files with latest versions?"
                } else {
                    "Update config files with newest versions?"
                })
                .interact()
                .into_diagnostic()?)
    {
        let mut updates: BTreeMap<PathBuf, BTreeMap<Id, UnresolvedVersionSpec>> = BTreeMap::new();

        for (id, item) in &items {
            updates
                .entry(item.config_source.clone())
                .or_default()
                .insert(
                    id.to_owned(),
                    if args.latest {
                        item.latest_version.to_unresolved_spec()
                    } else {
                        item.newest_version.to_unresolved_spec()
                    },
                );
        }

        for (config_path, updated_versions) in updates {
            println!(
                "Updating {} with {} versions",
                color::path(&config_path),
                updated_versions.len()
            );

            debug!(
                config = ?config_path,
                versions = ?updated_versions
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect::<BTreeMap<_, _>>(),
                "Updating config with versions",
            );

            ProtoConfig::update(config_path, |config| {
                config
                    .versions
                    .get_or_insert(Default::default())
                    .extend(updated_versions);
            })?;
        }

        println!(
            "Update complete! Run {} to install these new versions.",
            color::shell("proto use")
        );
    }

    Ok(())
}
