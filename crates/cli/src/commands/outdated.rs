use crate::error::ProtoCliError;
use crate::session::ProtoSession;
use clap::Args;
use iocraft::prelude::{element, Size};
use miette::IntoDiagnostic;
use proto_core::{Id, ProtoConfig, UnresolvedVersionSpec, VersionSpec, PROTO_PLUGIN_KEY};
use rustc_hash::FxHashSet;
use semver::VersionReq;
use serde::Serialize;
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_utils::json;
use std::collections::BTreeMap;
use std::path::PathBuf;
use tokio::spawn;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct OutdatedArgs {
    #[arg(
        long,
        help = "When updating versions, use the latest version instead of newest"
    )]
    latest: bool,

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
        UnresolvedVersionSpec::Calendar(version) => UnresolvedVersionSpec::Req(
            VersionReq::parse(format!("~{}", version.major).as_str()).unwrap(),
        ),
        UnresolvedVersionSpec::Semantic(version) => UnresolvedVersionSpec::Req(
            VersionReq::parse(format!("~{}", version.major).as_str()).unwrap(),
        ),
        _ => spec.clone(),
    }
}

#[tracing::instrument(skip_all)]
pub async fn outdated(session: ProtoSession, args: OutdatedArgs) -> AppResult {
    let env = &session.env;
    let manager = env.load_config_manager()?;

    debug!("Determining outdated tools based on config...");

    let mut configured_tools = BTreeMap::default();

    for file in manager.files.iter().rev() {
        if !file.exists
            || !env.config_mode.includes_global() && file.global
            || env.config_mode.only_local()
                && file.path.parent().is_none_or(|p| p != env.working_dir)
        {
            continue;
        }

        if let Some(file_versions) = &file.config.versions {
            for (tool_id, config_version) in file_versions {
                if tool_id == PROTO_PLUGIN_KEY {
                    continue;
                }

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

            let current_version = tool
                .resolve_version_candidate(&version_resolver, &config_version, true)
                .await?;
            let newest_range = get_in_major_range(&config_version);

            debug!(
                id = tool.id.as_str(),
                range = newest_range.to_string(),
                "Resolving newest version"
            );

            let newest_version = tool
                .resolve_version_candidate(&version_resolver, &newest_range, false)
                .await?;

            debug!(
                id = tool.id.as_str(),
                alias = initial_version.to_string(),
                "Resolving latest version"
            );

            let latest_version = tool
                .resolve_version_candidate(&version_resolver, &initial_version, true)
                .await?;

            Result::<_, miette::Report>::Ok((
                tool.id.clone(),
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

    if session.should_print_json() {
        session
            .console
            .out
            .write_line(json::format(&items, true)?)?;

        return Ok(None);
    }

    session.console.render(element! {
        Container {
            Table(
                headers: vec![
                    TableHeader::new("Tool", Size::Percent(10.0)),
                    TableHeader::new("Current", Size::Percent(8.0)),
                    TableHeader::new("Newest", Size::Percent(8.0)),
                    TableHeader::new("Latest", Size::Percent(8.0)),
                    TableHeader::new("Config", Size::Percent(66.0)),
                ]
            ) {
                #(items.iter().enumerate().map(|(i, (id, item))| {
                    element! {
                        TableRow(row: i as i32) {
                            TableCol(col: 0) {
                                StyledText(
                                    content: id.to_string(),
                                    style: Style::Id
                                )
                            }
                            TableCol(col: 1) {
                                StyledText(
                                    content: item.current_version.to_string(),
                                )
                            }
                            TableCol(col: 2) {
                                StyledText(
                                    content: item.newest_version.to_string(),
                                    style: if item.newest_version == item.current_version {
                                        Style::MutedLight
                                    } else {
                                        Style::Success
                                    }
                                )
                            }
                            TableCol(col: 3) {
                                StyledText(
                                    content: item.latest_version.to_string(),
                                    style: if item.latest_version == item.current_version {
                                        Style::MutedLight
                                    } else if item.latest_version == item.newest_version {
                                        Style::Success
                                    } else {
                                        Style::Failure
                                    }
                                )
                            }
                            TableCol(col: 4) {
                                StyledText(
                                    content: item.config_source.to_string_lossy(),
                                    style: Style::Path
                                )
                            }
                        }
                    }
                }))
            }
        }
    })?;

    // If updating versions, batch the changes based on config paths
    if !args.update {
        return Ok(None);
    }

    let skip_prompts = session.should_skip_prompts();
    let mut confirmed = false;

    if !skip_prompts {
        session
            .console
            .render_interactive(element! {
                Confirm(
                    label: if args.latest {
                        "Update config files with latest versions?"
                    } else {
                        "Update config files with newest versions?"
                    },
                    on_confirm: &mut confirmed,
                )
            })
            .await?;
    }

    if skip_prompts || confirmed {
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

        session.console.render(element! {
            Notice(variant: Variant::Success) {
                StyledText(
                    content: "Update complete! Run <shell>proto install</shell> to install these new versions."
                )
            }
        })?;
    }

    Ok(None)
}
