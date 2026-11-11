use crate::components::*;
use crate::session::{LoadToolOptions, ProtoSession, SessionResult};
use clap::Args;
use iocraft::prelude::element;
use proto_core::flow::locate::Locator as LocatorFlow;
use proto_core::{
    ConfigMode, Id, PluginLocator, ProtoToolConfig, ToolContext, ToolManifest, ToolMetadata,
    TrustedPluginRecord, UnresolvedVersionSpec, UntrustedPluginRecord,
    flow::locate::ExecutableLocation,
};
use serde::Serialize;
use starbase_console::ui::*;
use std::collections::BTreeMap;
use std::path::PathBuf;
use tracing::instrument;

#[derive(Serialize)]
struct PluginInfoOutput {
    bins: Vec<ExecutableLocation>,
    config: ProtoToolConfig,
    context: ToolContext,
    exe_file: Option<PathBuf>,
    exes_dirs: Vec<PathBuf>,
    globals_dirs: Vec<PathBuf>,
    globals_prefix: Option<String>,
    id: Id,
    inventory_dir: PathBuf,
    manifest: ToolManifest,
    metadata: ToolMetadata,
    name: String,
    plugin: PluginLocator,
    requires: Vec<ToolContext>,
    shims: Vec<ExecutableLocation>,
    trusted_plugin: Option<TrustedPluginRecord>,
    untrusted_plugin: Option<UntrustedPluginRecord>,
}

#[derive(Args, Clone, Debug)]
pub struct PluginInfoArgs {
    #[arg(
        required = true,
        help = "ID of plugin, optionally prefixed with a backend (for example, asdf:golang)"
    )]
    context: ToolContext,
}

#[instrument(skip(session))]
pub async fn info(session: ProtoSession, args: PluginInfoArgs) -> SessionResult {
    let global_config = session.load_config_with_mode(ConfigMode::Global)?;

    let tool = session
        .load_tool_with_options(
            &args.context,
            LoadToolOptions {
                detect_version: true,
                inherit_local: true,
                inherit_remote: Some(UnresolvedVersionSpec::default()),
                ..Default::default()
            },
        )
        .await?;
    let spec = tool.spec.clone();
    let requires = tool.get_requires()?;

    let mut locator = LocatorFlow::new(&tool, &spec);
    let bins = locator.locate_bins(None).await?;
    let shims = locator.locate_shims().await?;
    let locations = locator.locate_all().await?;

    let untrusted_plugin = session.env.get_untrusted_plugin(&tool.context, tool.ty)?;
    let trusted_plugin = tool
        .locator
        .as_ref()
        .map(|locator| {
            session
                .env
                .get_trusted_plugin(&tool.context, tool.ty, locator)
        })
        .transpose()?;
    let has_debug_file = trusted_plugin
        .as_ref()
        .is_some_and(|plugin| plugin.debug_file.is_some());

    if session.is_json_format() {
        session.console.write_json_for_format(PluginInfoOutput {
            bins,
            config: tool.config.clone(),
            context: tool.context.clone(),
            exe_file: locations.exe_file,
            exes_dirs: locations.exes_dirs,
            globals_dirs: locations.globals_dirs,
            globals_prefix: locations.globals_prefix,
            inventory_dir: tool.get_inventory_dir().to_path_buf(),
            shims,
            id: tool.get_id().clone(),
            name: tool.metadata.name.clone(),
            manifest: tool.inventory.manifest.clone(),
            metadata: tool.metadata.clone(),
            plugin: tool.locator.clone().unwrap(),
            requires,
            trusted_plugin,
            untrusted_plugin,
        })?;

        return Ok(None);
    }

    // PLUGIN

    let is_backend = tool.is_backend_plugin();
    let is_tool = !is_backend;

    session.console.render(element! {
        Container {
            Section(title: "Plugin") {
                Entry(
                    name: "ID",
                    value: element! {
                        StyledText(
                            content: tool.context.to_string(),
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
                    content: if is_backend {
                        "Backend".to_string()
                    } else {
                        "Tool".to_string()
                    },
                )
                #(is_tool.then(|| {
                    element! {
                        Entry(
                            name: "Category",
                            content: format!("{:?}", tool.metadata.type_of),
                        )
                    }
                }))
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
                #(tool.locator.as_ref().map(|loc| {
                    element! {
                        Locator(value: loc)
                    }
                }))
                #(if requires.is_empty() {
                    None
                } else {
                    Some(element! {
                        Entry(name: "Requires") {
                            List {
                                #(requires.iter().map(|req_context| {
                                    element! {
                                        ListItem {
                                            StyledText(
                                                content: req_context.to_string(),
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
                #(has_debug_file.then(|| {
                    element! {
                        Entry(
                            name: "Debug build?",
                            content: "Local WASM file replaced the built-in plugin".to_owned(),
                        )
                    }
                }))
                #(untrusted_plugin.as_ref().map(|plugin| {
                    element! {
                        Entry(
                            name: "Untrusted override?",
                            value: element! {
                                StyledText(
                                    content: plugin.get_display_locator(),
                                    style: Style::Url
                                )
                            }.into_any()
                        ) {
                            View {
                                StyledText(
                                    content: format!(
                                        "Override in <path>{0}</path> not used, as the config is not trusted. Run <shell>proto trust {0}</shell> to use it.",
                                        plugin.config.display(),
                                    ),
                                )
                            }
                        }
                    }
                }))
            }
        }
    })?;

    // INVENTORY

    session.console.render(element! {
        Container {
            Section(title: "Inventory") {
                Entry(
                    name: "Detected version",
                    value: element! {
                        StyledText(
                            content: tool.spec.get_resolved_version().to_string(),
                            style: Style::Hash
                        )
                    }.into_any()
                )
                #(global_config.versions.get(&tool.context).map(|version| {
                    element! {
                        Entry(
                            name: "Fallback version",
                            value: element! {
                                StyledText(
                                    content: version.to_string(),
                                    style: Style::Shell
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
                #(locations.exe_file.map(|file| {
                    element! {
                        Entry(
                            name: "Executable file",
                            value: element! {
                                StyledText(
                                    content: file.to_string_lossy(),
                                    style: Style::Path
                                )
                            }.into_any()
                        )
                    }
                }))
                Entry(
                    name: "Executables directories",
                    no_children: locations.exes_dirs.is_empty()
                ) {
                    List {
                        #(locations.exes_dirs.into_iter().map(|dir| {
                            element! {
                                ListItem {
                                    StyledText(
                                        content: dir.to_string_lossy(),
                                        style: Style::Path
                                    )
                                }
                            }
                        }))
                    }
                }
                #(locations.globals_prefix.map(|prefix| {
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
                #(locations.globals_dir.map(|dir| {
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
                        default_version: global_config.versions.get(&tool.context).map(|spec| &spec.req),
                        inventory: &tool.inventory,
                        versions: tool.installed_versions.iter().collect::<Vec<_>>(),
                    )
                }
                Entry(
                    name: "Remote aliases",
                    no_children: tool.remote_aliases.is_empty()
                ) {
                    SpecAliasesMap(
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
                    SpecAliasesMap(
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
