use crate::components::{Locator, VersionsMap};
use crate::session::ProtoSession;
use clap::Args;
use iocraft::prelude::element;
use proto_core::{Id, PluginLocator, ProtoToolConfig, ToolManifest, UnresolvedVersionSpec};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::Serialize;
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_utils::json;

#[derive(Serialize)]
pub struct PluginItem<'a> {
    name: String,
    locator: Option<PluginLocator>,
    config: Option<&'a ProtoToolConfig>,
    manifest: ToolManifest,
}

#[derive(Args, Clone, Debug)]
pub struct ListPluginsArgs {
    #[arg(help = "ID of plugins to list")]
    ids: Vec<Id>,

    #[arg(long, help = "Include resolved aliases in the output")]
    aliases: bool,

    #[arg(long, help = "Print the list in JSON format")]
    json: bool,

    #[arg(long, help = "Include installed versions in the output")]
    versions: bool,
}

#[tracing::instrument(skip_all)]
pub async fn list(session: ProtoSession, args: ListPluginsArgs) -> AppResult {
    let mut config = session.env.load_config()?.to_owned();
    let global_config = session.env.load_config_manager()?.get_global_config()?;

    let mut tools = session
        .load_tools_with_filters(FxHashSet::from_iter(&args.ids))
        .await?;

    tools.sort_by(|a, d| a.id.cmp(&d.id));

    // --json
    if args.json {
        let items = tools
            .into_iter()
            .map(|t| {
                let tool_config = config.tools.get(&t.id);
                let name = t.get_name().to_owned();

                (
                    t.id,
                    PluginItem {
                        name,
                        locator: t.locator,
                        config: tool_config,
                        manifest: t.inventory.manifest,
                    },
                )
            })
            .collect::<FxHashMap<_, _>>();

        session
            .console
            .out
            .write_line(json::format(&items, true)?)?;

        return Ok(None);
    }

    let latest_version = UnresolvedVersionSpec::default();

    for tool in tools {
        let tool_config = config.tools.remove(&tool.id).unwrap_or_default();

        let mut version_resolver = tool.load_version_resolver(&latest_version).await?;
        version_resolver.aliases.extend(tool_config.aliases);

        let mut versions = tool
            .inventory
            .manifest
            .installed_versions
            .iter()
            .collect::<Vec<_>>();
        versions.sort();

        session.console.render(element! {
            Container {
                Section(title: &tool.metadata.name) {
                    Entry(
                        name: "ID",
                        value: element! {
                            StyledText(
                                content: tool.id.to_string(),
                                style: Style::Id
                            )
                        }.into_any()
                    )

                    #(tool.locator.as_ref().map(|locator| {
                        element! {
                            Locator(value: locator)
                        }
                    }))

                    Entry(
                        name: "Store directory",
                        value: element! {
                            StyledText(
                                content: tool.get_inventory_dir().to_string_lossy(),
                                style: Style::Path
                            )
                        }.into_any()
                    )

                    #(if args.aliases {
                        Some(element! {
                            Entry(
                                name: "Aliases",
                                no_children: version_resolver.aliases.is_empty()
                            ) {
                                Map {
                                    #(version_resolver.aliases.iter().map(|(alias, version)| {
                                        element! {
                                            MapItem(
                                                name: element! {
                                                    StyledText(
                                                        content: alias,
                                                        style: Style::Id
                                                    )
                                                }.into_any(),
                                                value: element! {
                                                    StyledText(
                                                        content: version.to_string(),
                                                        style: Style::Hash
                                                    )
                                                }.into_any()
                                            )
                                        }
                                    }))
                                }
                            }
                        })
                    } else {
                        None
                    })

                    #(if args.versions {
                        Some(element! {
                            Entry(
                                name: "Versions",
                                no_children: versions.is_empty()
                            ) {
                                VersionsMap(
                                    default_version: global_config.versions.get(&tool.id),
                                    inventory: &tool.inventory,
                                    versions,
                                )
                            }
                        })
                    } else {
                        None
                    })
                }
            }
        })?;
    }

    Ok(None)
}
