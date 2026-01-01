use crate::components::create_datetime;
use crate::session::{LoadToolOptions, ProtoSession};
use clap::Args;
use indexmap::IndexMap;
use iocraft::prelude::{View, element};
use proto_core::{ToolContext, ToolSpec, VersionSpec};
use serde::Serialize;
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_utils::json;
use std::collections::BTreeMap;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct VersionsArgs {
    #[arg(required = true, help = "Tool to list for")]
    context: ToolContext,

    #[arg(long, help = "Include aliases in the output")]
    aliases: bool,

    #[arg(long, help = "Only display installed versions")]
    installed: bool,
}

#[derive(Serialize)]
pub struct VersionItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    installed_at: Option<u128>,
    version: VersionSpec,
}

#[derive(Serialize)]
pub struct VersionsResult {
    versions: Vec<VersionItem>,
    local_aliases: BTreeMap<String, ToolSpec>,
    remote_aliases: BTreeMap<String, ToolSpec>,
}

#[tracing::instrument(skip_all)]
pub async fn versions(session: ProtoSession, args: VersionsArgs) -> AppResult {
    let tool = session
        .load_tool_with_options(
            &args.context,
            LoadToolOptions {
                inherit_local: true,
                inherit_remote: true,
                ..Default::default()
            },
        )
        .await?;

    debug!("Loading versions from remote");

    if tool.remote_versions.is_empty() {
        session.console.render(element! {
            Notice(variant: Variant::Failure) {
                StyledText(
                    content: "No versions available from remote registry"
                )
            }
        })?;

        return Ok(Some(1));
    }

    let versions = tool
        .remote_versions
        .iter()
        .filter_map(|version| {
            let installed_at = tool
                .inventory
                .manifest
                .versions
                .get(version)
                .map(|meta| meta.installed_at);

            if args.installed && installed_at.is_none() {
                None
            } else {
                Some(VersionItem {
                    installed_at,
                    version: version.to_owned(),
                })
            }
        })
        .collect::<Vec<_>>();

    if session.should_print_json() {
        let result = VersionsResult {
            versions,
            local_aliases: tool.local_aliases,
            remote_aliases: tool.remote_aliases,
        };

        session
            .console
            .out
            .write_line(json::format(&result, true)?)?;

        return Ok(None);
    }

    let mut aliases = IndexMap::<&String, &ToolSpec>::default();

    if args.aliases && !args.installed {
        aliases.extend(&tool.remote_aliases);
        aliases.extend(&tool.local_aliases);
    }

    session.console.render(element! {
        Container {
            #(versions.into_iter().map(|item| {
                element! {
                    View {
                        #(if let Some(timestamp) = item.installed_at {
                            element! {
                                StyledText(
                                    content: format!(
                                        "<shell>{}</shell> <muted>-</muted> <mutedlight>installed {}</mutedlight>",
                                        item.version,
                                        create_datetime(timestamp).unwrap_or_default().format("%x")
                                    ),
                                )
                            }
                        } else {
                            element! {
                                StyledText(content: item.version.to_string())
                            }
                        })
                    }
                }
            }))

            #(aliases.into_iter().map(|(alias, version)| {
                element! {
                    View {
                        StyledText(content: format!("{alias} <muted>→</muted> {}", version.req))
                    }
                }
            }))
        }
    })?;

    Ok(None)
}
