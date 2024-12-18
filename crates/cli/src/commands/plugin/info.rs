use crate::components::*;
use crate::session::{LoadToolOptions, ProtoSession};
use clap::Args;
use iocraft::prelude::element;
use proto_core::{
    flow::locate::ExecutableLocation, ConfigMode, Id, PluginLocator, ProtoToolConfig, ToolManifest,
};
use proto_pdk_api::ToolMetadataOutput;
use serde::Serialize;
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_utils::json;
use std::collections::BTreeMap;
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
    let global_config = session.load_config_with_mode(ConfigMode::Global)?;

    let mut tool = session
        .load_tool_with_options(
            &args.id,
            LoadToolOptions {
                detect_version: true,
                inherit_local: true,
                inherit_remote: true,
                ..Default::default()
            },
        )
        .await?;

    let bins = tool.resolve_bin_locations(true).await?;
    let shims = tool.resolve_shim_locations().await?;

    if args.json {
        let info = PluginInfo {
            bins,
            config: tool.config.clone(),
            exe_file: tool.locate_exe_file().await?,
            exes_dir: tool.locate_exes_dir().await?,
            globals_dirs: tool.locate_globals_dirs().await?,
            globals_prefix: tool.locate_globals_prefix().await?,
            inventory_dir: tool.get_inventory_dir(),
            shims,
            id: tool.id.clone(),
            name: tool.metadata.name.clone(),
            manifest: tool.inventory.manifest.clone(),
            metadata: tool.metadata.clone(),
            plugin: tool.locator.clone().unwrap(),
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
                #(global_config.versions.get(&tool.id).map(|version| {
                    element! {
                        Entry(
                            name: "Fallback version",
                            value: element! {
                                StyledText(
                                    content: version.to_string(),
                                    style: Style::Hash
                                )
                            }.into_any()
                        )
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
                    no_children: tool.installed_versions.is_empty()
                ) {
                    VersionsMap(
                        default_version: global_config.versions.get(&tool.id),
                        inventory: &tool.inventory,
                        versions: tool.installed_versions.iter().collect::<Vec<_>>(),
                    )
                }
                Entry(
                    name: "Remote aliases",
                    no_children: tool.remote_aliases.is_empty()
                ) {
                    AliasesMap(
                        aliases: tool.remote_aliases.iter().collect::<BTreeMap<_, _>>()
                    )
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
                    no_children: tool.local_aliases.is_empty()
                ) {
                    AliasesMap(
                        aliases: tool.local_aliases.iter().collect::<BTreeMap<_, _>>()
                    )
                }
                Entry(
                    name: "Environment variables",
                    no_children: tool.config.env.is_empty()
                ) {
                    Map {
                        #(tool.config.env.iter().map(|(key, value)| {
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
                    no_children: tool.config.config.is_empty()
                ) {
                    Map {
                        #(tool.config.config.iter().map(|(key, value)| {
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
