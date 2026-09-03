use crate::error::ProtoCliError;
use crate::session::{LoadToolOptions, ProtoSession, SessionResult};
use clap::Args;
use iocraft::prelude::{Size, element};
use miette::IntoDiagnostic;
use proto_core::flow::lock::Locker;
use proto_core::flow::resolve::{ProtoResolveError, Resolver};
use proto_core::{
    PROTO_CONFIG_NAME, ProtoConfig, Requirement, ToolContext, ToolSpec, UnresolvedVersionSpec,
    VersionSpec, cfg, reporter::NoticeOutput,
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
    locked_version: Option<VersionSpec>,
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

fn render_table(
    session: &ProtoSession,
    items: &BTreeMap<ToolContext, OutdatedItem>,
) -> miette::Result<()> {
    let ctx_width = items.keys().fold(0, |acc, ctx| acc.max(ctx.as_str().len()));

    // Only show the locked column if a tool is using a lockfile
    let show_locked = items.values().any(|item| item.locked_version.is_some());

    let mut headers = vec![
        TableHeader::new("Tool", Size::Length((ctx_width + 3).max(10) as u32)),
        TableHeader::new("Current", Size::Length(10)),
    ];

    if show_locked {
        headers.push(TableHeader::new("Locked", Size::Length(10)));
    }

    headers.extend([
        TableHeader::new("Newest", Size::Length(10)),
        TableHeader::new("Latest", Size::Length(10)),
        TableHeader::new("Config", Size::Auto),
    ]);

    session.console.table(
        headers,
        items
            .iter()
            .map(|(ctx, item)| {
                let mut row = vec![format!("<id>{ctx}</id>"), item.current_version.to_string()];

                if show_locked {
                    row.push(if let Some(version) = &item.locked_version {
                        format!("<hash>{version}</hash>")
                    } else {
                        "<mutedlight>N/A</mutedlight>".into()
                    });
                }

                row.extend([
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
                ]);

                row
            })
            .collect(),
    )?;

    Ok(())
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

            let config_version = tool.detected_version.as_ref().unwrap();

            debug!(
                tool = tool.context.as_str(),
                config = config_version.to_string(),
                "Resolving current version"
            );

            let current_version = Resolver::new(&tool)
                .resolve_version_candidate(&config_version.req, true, true)
                .await?;
            let newest_range = get_in_major_range(&config_version.req);

            debug!(
                tool = tool.context.as_str(),
                range = newest_range.to_string(),
                "Resolving newest version"
            );

            let newest_version = Resolver::new(&tool)
                .resolve_version_candidate(&newest_range, false, true)
                .await?;

            debug!(tool = tool.context.as_str(), "Resolving latest version");

            let latest_version = Resolver::new(&tool)
                .resolve_version_candidate(&UnresolvedVersionSpec::default(), true, true)
                .await?;

            // If a lockfile record exists for the configured spec, then the
            // version is locked, and installs will use it instead
            let locked_version = if let Some(record) = &tool.spec.version_locked {
                record.version.clone()
            } else {
                Locker::new(&tool)
                    .resolve_locked_record(config_version)
                    .ok()
                    .flatten()
                    .and_then(|record| record.version)
            };

            let item = OutdatedItem {
                is_latest: current_version == latest_version,
                is_outdated: newest_version > current_version || latest_version > current_version,
                config_source: tool.detected_source.clone(),
                config_version: config_version.to_owned(),
                current_version,
                locked_version,
                newest_version,
                latest_version,
            };

            Result::<_, ProtoResolveError>::Ok((tool, item))
        }));
    }

    let mut items = BTreeMap::default();
    let mut tools = vec![];

    while let Some(result) = set.join_next().await {
        let (tool, item) = result.into_diagnostic()??;

        items.insert(tool.context.clone(), item);
        tools.push(tool);
    }

    if items.is_empty() {
        return Err(ProtoCliError::NoConfiguredTools.into());
    }

    debug!(
        tools = ?items.keys().map(|ctx| ctx.as_str()).collect::<Vec<_>>(),
        "Found tools with configured versions, loading them",
    );

    if session.is_json_format() {
        session.console.write_json_for_format(&items)?;
    } else {
        render_table(&session, &items)?;
    }

    // If updating versions, batch the changes based on config paths
    if !args.update {
        return Ok(None);
    }

    // A prompt can only be answered when attached to a terminal that isn't
    // rendering machine readable output, otherwise the explicit `--update`
    // flag is the confirmation, as there's no one to ask
    let can_prompt =
        !session.should_skip_prompts() && !session.is_json_format() && session.is_tty();

    if can_prompt {
        let mut confirmed = false;

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

        if !confirmed {
            session.console.notice(
                Variant::Info,
                "Update aborted, no configuration files were changed.",
            )?;

            return Ok(None);
        }
    }

    let mut updates: BTreeMap<PathBuf, BTreeMap<ToolContext, UnresolvedVersionSpec>> =
        BTreeMap::new();
    let mut skipped = vec![];

    for (context, item) in &items {
        let Some(src) = &item.config_source else {
            skipped.push(format!(
                "<id>{context}</id> - version was not detected from a configuration file"
            ));

            continue;
        };

        // Only proto configs can be updated, including environment scoped
        // configs, as versions may also be detected from ecosystem files
        if !ProtoConfig::is_config_file(src) {
            warn!(
                config = ?src,
                "Unable to update the version for {}, as its config source is not a {} file",
                color::id(context),
                color::file(PROTO_CONFIG_NAME),
            );

            skipped.push(format!(
                "<id>{context}</id> - <path>{}</path> is not a <file>{PROTO_CONFIG_NAME}</file> file",
                src.to_string_lossy()
            ));

            continue;
        }

        // Don't update aliases, only semantic or calendar versions
        if matches!(
            item.config_version.req,
            UnresolvedVersionSpec::Canary | UnresolvedVersionSpec::Alias(_)
        ) {
            skipped.push(format!(
                "<id>{context}</id> - <version>{}</version> is an alias, not a version",
                item.config_version.req
            ));

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

    // Don't silently do nothing, as callers have no other signal that we
    // declined to write anything to their configs
    if updates.is_empty() {
        let mut messages = vec!["No versions were updated!".into()];
        messages.extend(skipped);

        session.console.notice_with(NoticeOutput {
            variant: Variant::Caution,
            messages,
            ..Default::default()
        })?;

        return Ok(None);
    }

    for (config_path, updated_versions) in &updates {
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
                doc[context.as_str()] =
                    cfg::value(ToolSpec::new(updated_version.to_owned()).to_string());
            }
        })?;
    }

    // Update records in the lockfiles owned by the updated configs,
    // otherwise the stale records will be used indefinitely
    for tool in &tools {
        let Some(item) = items.get(&tool.context) else {
            continue;
        };

        let Some(src) = &item.config_source else {
            continue;
        };

        let Some(new_spec) = updates
            .get(src)
            .and_then(|versions| versions.get(&tool.context))
        else {
            continue;
        };

        Locker::for_config(tool, src).update_spec_in_lockfile(
            &item.config_version.req,
            new_spec,
            if args.latest {
                &item.latest_version
            } else {
                &item.newest_version
            },
        )?;
    }

    let mut messages = vec![
        "Update complete! Run <shell>proto install</shell> to install these new versions.".into(),
    ];

    if !skipped.is_empty() {
        messages.push("The following tools were not updated:".into());
        messages.extend(skipped);
    }

    session.console.notice_with(NoticeOutput {
        variant: Variant::Success,
        messages,
        ..Default::default()
    })?;

    Ok(None)
}
