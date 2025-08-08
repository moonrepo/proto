use crate::components::{Locator, SpecAliasesMap, VersionsMap};
use crate::session::{LoadToolOptions, ProtoSession};
use clap::Args;
use iocraft::prelude::element;
use proto_core::{ConfigMode, Id, PluginLocator, ProtoToolConfig, ToolContext, ToolManifest};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::Serialize;
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_utils::json;
use std::collections::BTreeMap;

#[derive(Serialize)]
struct PluginItem {
    name: String,
    locator: Option<PluginLocator>,
    config: ProtoToolConfig,
    manifest: ToolManifest,
}

#[derive(Args, Clone, Debug)]
pub struct ListPluginsArgs {
    #[arg(help = "ID of plugins to list")]
    ids: Vec<Id>,

    #[arg(long, help = "Include resolved aliases in the output")]
    aliases: bool,

    #[arg(long, help = "Include installed versions in the output")]
    versions: bool,
}

#[tracing::instrument(skip_all)]
pub async fn list(session: ProtoSession, args: ListPluginsArgs) -> AppResult {
    let global_config = session.load_config_with_mode(ConfigMode::Global)?;

    let mut tools = session
        .load_tools_with_options(LoadToolOptions {
            tools: FxHashSet::from_iter(if args.ids.is_empty() {
                // Use plugins instead of versions since we want to
                // list all plugins currently in use, even built-ins
                global_config
                    .plugins
                    .keys()
                    .map(|id| ToolContext::new(id.to_owned()))
                    .collect::<Vec<_>>()
            } else {
                args.ids
                    .clone()
                    .into_iter()
                    .map(ToolContext::new)
                    .collect::<Vec<_>>()
            }),
            inherit_local: true,
            inherit_remote: true,
            ..Default::default()
        })
        .await?;

    tools.sort_by(|a, d| a.context.cmp(&d.context));

    if session.should_print_json() {
        let items = tools
            .into_iter()
            .map(|tool| {
                (
                    tool.context.clone(),
                    PluginItem {
                        name: tool.get_name().to_owned(),
                        locator: tool.locator.clone(),
                        config: tool.config.clone(),
                        manifest: tool.inventory.manifest.clone(),
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

    for tool in tools {
        let mut aliases = BTreeMap::default();
        aliases.extend(&tool.remote_aliases);
        aliases.extend(&tool.local_aliases);

        let is_tool = !tool.is_backend_plugin().await;

        session.console.render(element! {
            Container {
                Section(title: &tool.metadata.name) {
                    Entry(
                        name: "ID",
                        value: element! {
                            StyledText(
                                content: tool.context.to_string(),
                                style: Style::Id
                            )
                        }.into_any()
                    )

                    #(tool.locator.as_ref().map(|locator| {
                        element! {
                            Locator(value: locator)
                        }
                    }))

                    #(is_tool.then(|| {
                        element! {
                            Entry(
                                name: "Store directory",
                                value: element! {
                                    StyledText(
                                        content: tool.get_inventory_dir().to_string_lossy(),
                                        style: Style::Path
                                    )
                                }.into_any()
                            )
                        }
                    }))

                    #(if args.aliases && is_tool {
                        Some(element! {
                            Entry(
                                name: "Aliases",
                                no_children: aliases.is_empty()
                            ) {
                                SpecAliasesMap(aliases)
                            }
                        })
                    } else {
                        None
                    })

                    #(if args.versions && is_tool {
                        Some(element! {
                            Entry(
                                name: "Versions",
                                no_children: tool.installed_versions.is_empty()
                            ) {
                                VersionsMap(
                                    default_version: global_config.versions.get(&tool.context).map(|spec| &spec.req),
                                    inventory: &tool.inventory,
                                    versions: tool.installed_versions.iter().collect::<Vec<_>>(),
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
