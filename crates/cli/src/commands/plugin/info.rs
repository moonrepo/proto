use crate::components::*;
use crate::session::ProtoSession;
use clap::Args;
use iocraft::prelude::*;
use proto_core::{
    detect_version, flow::locate::ExecutableLocation, Id, PluginLocator, ProtoToolConfig,
    ToolManifest, UnresolvedVersionSpec,
};
use proto_pdk_api::ToolMetadataOutput;
use serde::Serialize;
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_utils::json;
use std::path::PathBuf;

#[derive(Serialize)]
struct PluginInfo {
    bins: Vec<ExecutableLocation>,
    config: ProtoToolConfig,
    exe_file: PathBuf,
    exes_dir: Option<PathBuf>,
    globals_dirs: Vec<PathBuf>,
    globals_prefix: Option<String>,
    id: Id,
    inventory_dir: PathBuf,
    manifest: ToolManifest,
    metadata: ToolMetadataOutput,
    name: String,
    plugin: PluginLocator,
    shims: Vec<ExecutableLocation>,
}

#[derive(Args, Clone, Debug)]
pub struct InfoPluginArgs {
    #[arg(required = true, help = "ID of plugin")]
    id: Id,

    #[arg(long, help = "Print the info in JSON format")]
    json: bool,
}

#[tracing::instrument(skip_all)]
pub async fn info(session: ProtoSession, args: InfoPluginArgs) -> AppResult {
    let mut tool = session.load_tool(&args.id).await?;
    let version = detect_version(&tool, None)
        .await
        .unwrap_or_else(|_| UnresolvedVersionSpec::parse("*").unwrap());

    tool.resolve_version(&version, false).await?;

    let global_config = session.env.load_config_manager()?.get_global_config()?;
    let mut config = session.env.load_config()?.to_owned();
    let tool_config = config.tools.remove(&tool.id).unwrap_or_default();
    let bins = tool.resolve_bin_locations(true).await?;
    let shims = tool.resolve_shim_locations().await?;

    if args.json {
        let info = PluginInfo {
            bins,
            config: tool_config,
            exe_file: tool.locate_exe_file().await?,
            exes_dir: tool.locate_exes_dir().await?,
            globals_dirs: tool.locate_globals_dirs().await?,
            globals_prefix: tool.locate_globals_prefix().await?,
            inventory_dir: tool.get_inventory_dir(),
            shims,
            id: tool.id,
            name: tool.metadata.name.clone(),
            manifest: tool.inventory.manifest,
            metadata: tool.metadata,
            plugin: tool.locator.unwrap(),
        };

        session.console.out.write_line(json::format(&info, true)?)?;

        return Ok(None);
    }

    // PLUGIN

    session.console.render(element! {
        Container {
            Section(title: "Plugin") {
                Entry(
                    name: "ID",
                    value: element! {
                        StyledText(
                            content: tool.id.to_string(),
                            style: Style::Id
                        )
                    }.into_any()
                )
                Entry(
                    name: "Name",
                    content: tool.metadata.name.clone(),
                )
                Entry(
                    name: "Type",
                    content: format!("{:?}", tool.metadata.type_of),
                )
                #(tool.metadata.plugin_version.as_ref().map(|version| {
                    element! {
                        Entry(
                            name: "Version",
                            value: element! {
                                StyledText(
                                    content: version.to_string(),
                                    style: Style::Hash
                                )
                            }.into_any()
                        )
                    }
                }))

                #(tool.locator.as_ref().map(|locator| {
                    element! {
                        Locator(value: locator)
                    }
                }))

                #(if tool.metadata.requires.is_empty() {
                    None
                } else {
                    Some(element! {
                        Entry(name: "Requires") {
                            List {
                                #(tool.metadata.requires.iter().map(|req_id| {
                                    element! {
                                        ListItem {
                                            StyledText(
                                                content: req_id,
                                                style: Style::Id
                                            )
                                        }
                                    }
                                }))
                            }
                        }
                    })
                })

                #(if tool.metadata.deprecations.is_empty() {
                    None
                } else {
                    Some(element! {
                        Entry(name: "Deprecations") {
                            List {
                                #(tool.metadata.deprecations.iter().map(|content| {
                                    element! {
                                        ListItem {
                                            StyledText(content)
                                        }
                                    }
                                }))
                            }
                        }
                    })
                })
            }
        }
    })?;

    // INVENTORY

    let exe_file = tool.locate_exe_file().await?;
    let exes_dir = tool.locate_exes_dir().await?;
    let globals_dir = tool.locate_globals_dir().await?;
    let globals_prefix = tool.locate_globals_prefix().await?;

    let version_resolver = tool
        .load_version_resolver(&UnresolvedVersionSpec::default())
        .await?;

    let mut versions = tool
        .inventory
        .manifest
        .installed_versions
        .iter()
        .collect::<Vec<_>>();
    versions.sort();

    session.console.render(element! {
        Container {
            Section(title: "Inventory") {
                Entry(
                    name: "Detected version",
                    value: element! {
                        StyledText(
                            content: tool.get_resolved_version().to_string(),
                            style: Style::Hash
                        )
                    }.into_any()
                )
                Entry(
                    name: "Store directory",
                    value: element! {
                        StyledText(
                            content: tool.get_inventory_dir().to_string_lossy(),
                            style: Style::Path
                        )
                    }.into_any()
                )
                Entry(
                    name: "Executable file",
                    value: element! {
                        StyledText(
                            content: exe_file.to_string_lossy(),
                            style: Style::Path
                        )
                    }.into_any()
                )
                #(exes_dir.map(|dir| {
                    element! {
                        Entry(
                            name: "Executables directory",
                            value: element! {
                                StyledText(
                                    content: dir.to_string_lossy(),
                                    style: Style::Path
                                )
                            }.into_any()
                        )
                    }
                }))
                #(globals_prefix.map(|prefix| {
                    element! {
                        Entry(
                            name: "Global packages prefix",
                            value: element! {
                                StyledText(
                                    content: prefix,
                                    style: Style::Property
                                )
                            }.into_any()
                        )
                    }
                }))
                #(globals_dir.map(|dir| {
                    element! {
                        Entry(
                            name: "Global packages directory",
                            value: element! {
                                StyledText(
                                    content: dir.to_string_lossy(),
                                    style: Style::Path
                                )
                            }.into_any()
                        )
                    }
                }))
                Entry(
                    name: "Shims",
                    no_children: shims.is_empty()
                ) {
                    List {
                        #(shims.into_iter().map(|shim| {
                            element! {
                                ListItem {
                                    StyledText(
                                        content: shim.path.to_string_lossy(),
                                        style: Style::Path
                                    )
                                }
                            }
                        }))
                    }
                }
                Entry(
                    name: "Binaries",
                    no_children: bins.is_empty()
                ) {
                    List {
                        #(bins.into_iter().map(|bin| {
                            element! {
                                ListItem {
                                    StyledText(
                                        content: bin.path.to_string_lossy(),
                                        style: Style::Path
                                    )
                                }
                            }
                        }))
                    }
                }
                Entry(
                    name: "Installed versions",
                    no_children: versions.is_empty()
                ) {
                    VersionsMap(
                        default_version: global_config.versions.get(&tool.id),
                        inventory: &tool.inventory,
                        versions,
                    )
                }
                Entry(
                    name: "Remote aliases",
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
            }
        }
    })?;

    // CONFIG

    session.console.render(element! {
        Container {
            Section(title: "Configuration") {
                Entry(
                    name: "Local aliases",
                    no_children: tool_config.aliases.is_empty()
                ) {
                    Map {
                        #(tool_config.aliases.iter().map(|(alias, version)| {
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
                Entry(
                    name: "Environment variables",
                    no_children: tool_config.env.is_empty()
                ) {
                    Map {
                        #(tool_config.env.iter().map(|(key, value)| {
                            element! {
                                MapItem(
                                    name: element! {
                                        StyledText(
                                            content: key,
                                            style: Style::Property
                                        )
                                    }.into_any(),
                                    value: element! {
                                        EnvVar(value: value)
                                    }.into_any()
                                )
                            }
                        }))
                    }
                }
                Entry(
                    name: "Settings",
                    no_children: tool_config.config.is_empty()
                ) {
                    Map {
                        #(tool_config.config.iter().map(|(key, value)| {
                            element! {
                                MapItem(
                                    name: element! {
                                        StyledText(
                                            content: key,
                                            style: Style::Property
                                        )
                                    }.into_any(),
                                    value: element! {
                                        StyledText(
                                            content: value.to_string(),
                                            style: Style::MutedLight
                                        )
                                    }.into_any()
                                )
                            }
                        }))
                    }
                }
            }
        }
    })?;

    Ok(None)
}
