use crate::error::ProtoCliError;
use crate::session::{LoadToolOptions, ProtoSession, SessionResult};
use clap::Args;
use iocraft::prelude::{Size, element};
use miette::IntoDiagnostic;
use proto_core::flow::resolve::{ProtoResolveError, Resolver};
use proto_core::{
    PROTO_CONFIG_NAME, ProtoConfig, Requirement, ToolContext, ToolSpec, UnresolvedVersionSpec,
    VersionSpec, cfg,
};
use serde::Serialize;
use starbase_console::ui::*;
use starbase_styles::color;
use std::collections::BTreeMap;
use std::path::PathBuf;
use tokio::task::JoinSet;
use tracing::{debug, instrument, warn};

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

#[derive(Debug, Serialize)]
pub struct OutdatedItem {
    is_latest: bool,
    is_outdated: bool,
    config_source: Option<PathBuf>,
    config_version: ToolSpec,
    current_version: VersionSpec,
    newest_version: VersionSpec,
    latest_version: VersionSpec,
}

fn get_in_major_range(spec: &UnresolvedVersionSpec) -> UnresolvedVersionSpec {
    match spec {
        UnresolvedVersionSpec::Version(version) => UnresolvedVersionSpec::Requirement(
            Requirement::parse(format!("~{}", version.major).as_str()).unwrap(),
        ),
        _ => spec.clone(),
    }
}

#[instrument(skip(session))]
pub async fn outdated(session: ProtoSession, args: OutdatedArgs) -> SessionResult {
    debug!("Determining outdated tools based on config...");

    let tools = session
        .load_all_tools_with_options(LoadToolOptions {
            detect_version: true,
            ..Default::default()
        })
        .await?;

    let mut set = JoinSet::new();

    for mut tool in tools {
        if tool.detected_version.is_none() {
            continue;
        }

        set.spawn(Box::pin(async move {
            tool.disable_caching();

            debug!("Checking {}", tool.get_name());

            let initial_version = UnresolvedVersionSpec::default(); // latest
            let config_version = tool.detected_version.as_ref().unwrap();
            let mut version_resolver = Resolver::new(&tool);

            version_resolver.load_versions(&initial_version).await?;

            debug!(
                tool = tool.context.as_str(),
                config = config_version.to_string(),
                "Resolving current version"
            );

            let current_version = version_resolver
                .resolve_version_candidate(&config_version.req, true)
                .await?;
            let newest_range = get_in_major_range(&config_version.req);

            debug!(
                tool = tool.context.as_str(),
                range = newest_range.to_string(),
                "Resolving newest version"
            );

            let newest_version = version_resolver
                .resolve_version_candidate(&newest_range, false)
                .await?;

            debug!(
                tool = tool.context.as_str(),
                alias = initial_version.to_string(),
                "Resolving latest version"
            );

            let latest_version = version_resolver
                .resolve_version_candidate(&initial_version, true)
                .await?;

            Result::<_, ProtoResolveError>::Ok((
                tool.context.clone(),
                OutdatedItem {
                    is_latest: current_version == latest_version,
                    is_outdated: newest_version > current_version
                        || latest_version > current_version,
                    config_source: tool.detected_source,
                    config_version: config_version.to_owned(),
                    current_version,
                    newest_version,
                    latest_version,
                },
            ))
        }));
    }

    let mut items = BTreeMap::default();

    while let Some(result) = set.join_next().await {
        let (context, item) = result.into_diagnostic()??;

        items.insert(context, item);
    }

    if items.is_empty() {
        return Err(ProtoCliError::NoConfiguredTools.into());
    }

    debug!(
        tools = ?items.keys().map(|ctx| ctx.as_str()).collect::<Vec<_>>(),
        "Found tools with configured versions, loading them",
    );

    if session.is_json_format() {
        session.console.write_json_for_format(items)?;

        return Ok(None);
    }

    let ctx_width = items.keys().fold(0, |acc, ctx| acc.max(ctx.as_str().len()));

    session.console.table(
        vec![
            TableHeader::new("Tool", Size::Length((ctx_width + 3).max(10) as u32)),
            TableHeader::new("Current", Size::Length(10)),
            TableHeader::new("Newest", Size::Length(10)),
            TableHeader::new("Latest", Size::Length(10)),
            TableHeader::new("Config", Size::Auto),
        ],
        items
            .iter()
            .map(|(ctx, item)| {
                vec![
                    format!("<id>{ctx}</id>"),
                    item.current_version.to_string(),
                    if item.newest_version == item.current_version {
                        format!("<mutedlight>{}</mutedlight>", item.newest_version)
                    } else {
                        format!("<success>{}</success>", item.newest_version)
                    },
                    if item.latest_version == item.current_version {
                        format!("<mutedlight>{}</mutedlight>", item.latest_version)
                    } else if item.latest_version == item.newest_version {
                        format!("<success>{}</success>", item.latest_version)
                    } else {
                        format!("<failure>{}</failure>", item.latest_version)
                    },
                    if let Some(src) = &item.config_source {
                        format!("<path>{}</path>", src.to_string_lossy())
                    } else {
                        "<mutedlight>N/A</mutedlight>".into()
                    },
                ]
            })
            .collect(),
    )?;

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
                        "Update config files with <label>latest</label> versions?"
                    } else {
                        "Update config files with <label>newest</label> versions?"
                    },
                    on_confirm: &mut confirmed,
                )
            })
            .await?;
    }

    if skip_prompts || confirmed {
        let mut updates: BTreeMap<PathBuf, BTreeMap<ToolContext, UnresolvedVersionSpec>> =
            BTreeMap::new();

        for (context, item) in &items {
            let Some(src) = &item.config_source else {
                continue;
            };

            if !src.ends_with(PROTO_CONFIG_NAME) {
                warn!(
                    config = ?src,
                    "Unable to update the version for {}, as its config source is not a {} file",
                    color::id(context),
                    color::file(PROTO_CONFIG_NAME),
                );

                continue;
            }

            // Don't update aliases, only semantic or calendar versions
            if matches!(
                item.config_version.req,
                UnresolvedVersionSpec::Canary | UnresolvedVersionSpec::Alias(_)
            ) {
                continue;
            }

            updates.entry(src.to_owned()).or_default().insert(
                context.to_owned(),
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

            ProtoConfig::update_document(config_path, |doc| {
                for (context, updated_version) in updated_versions {
                    doc[context.as_str()] = cfg::value(ToolSpec::new(updated_version).to_string());
                }
            })?;
        }

        session.console.notice(
            Variant::Success,
            "Update complete! Run <shell>proto install</shell> to install these new versions.",
        )?;
    }

    Ok(None)
}
