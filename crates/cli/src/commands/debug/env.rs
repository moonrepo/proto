use crate::session::ProtoSession;
use clap::Args;
use iocraft::prelude::*;
use proto_core::layout::Store;
use proto_pdk_api::{HostArch, HostOS};
use serde::Serialize;
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_utils::json;
use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;

#[derive(Serialize)]
struct EnvironmentInfo {
    arch: HostArch,
    configs: Vec<PathBuf>,
    os: HostOS,
    proto_version: String,
    vars: BTreeMap<String, String>,
    virtual_paths: BTreeMap<PathBuf, PathBuf>,
}

#[derive(Serialize)]
struct DebugEnvResult<'a> {
    store: &'a Store,
    env: EnvironmentInfo,
}

#[derive(Args, Clone, Debug)]
pub struct DebugEnvArgs {
    #[arg(long, help = "Print the data in JSON format")]
    json: bool,
}

#[tracing::instrument(skip_all)]
pub async fn env(session: ProtoSession, args: DebugEnvArgs) -> AppResult {
    let env = &session.env;
    let manager = env.load_config_manager()?;

    let environment = EnvironmentInfo {
        arch: HostArch::from_env(),
        configs: manager
            .files
            .iter()
            .filter_map(|file| {
                if file.exists {
                    Some(file.path.to_path_buf())
                } else {
                    None
                }
            })
            .collect(),
        os: HostOS::from_env(),
        proto_version: env!("CARGO_PKG_VERSION").into(),
        vars: env::vars()
            .filter_map(|(k, v)| {
                if k.starts_with("PROTO_") {
                    Some((k, v))
                } else {
                    None
                }
            })
            .collect(),
        virtual_paths: env.get_virtual_paths(),
    };

    if args.json {
        let result = DebugEnvResult {
            store: &env.store,
            env: environment,
        };

        session
            .console
            .out
            .write_line(json::format(&result, true)?)?;

        return Ok(None);
    }

    let store_paths = vec![
        ("Root", &env.store.dir),
        ("Bins", &env.store.bin_dir),
        ("Shims", &env.store.shims_dir),
        ("Plugins", &env.store.plugins_dir),
        ("Tools", &env.store.inventory_dir),
        ("Temp", &env.store.temp_dir),
    ];

    session.console.render(element! {
        Container {
            Section(title: "Store") {
                #(store_paths.into_iter().map(|(name, path)| {
                    element! {
                        Entry(
                            name,
                            value: element! {
                                StyledText(content: path.to_string_lossy(), style: Style::Path)
                            }.into_any(),
                        )
                    }
                }))
            }
            Section(title: "Environment") {
                Entry(
                    name: "Proto version",
                    content: environment.proto_version,
                )
                Entry(
                    name: "Operating system",
                    content: environment.os.to_string(),
                )
                Entry(
                    name: "Architecture",
                    content: environment.arch.to_string(),
                )
                Entry(
                    name: "Config sources",
                    no_children: environment.configs.is_empty(),
                ) {
                    List {
                        #(environment.configs.into_iter().map(|file| {
                            element! {
                                ListItem {
                                    StyledText(
                                        content: file.to_string_lossy(),
                                        style: Style::Path,
                                    )
                                }
                            }
                        }))
                    }
                }
                Entry(
                    name: "Virtual paths",
                    no_children: environment.virtual_paths.is_empty(),
                ) {
                    Map {
                        #(environment.virtual_paths.into_iter().map(|(host, guest)| {
                            let name = element! {
                                StyledText(
                                    content: guest.to_string_lossy(),
                                    style: Style::File
                                )
                            }.into_any();

                            let value = element! {
                                StyledText(
                                    content: host.to_string_lossy(),
                                    style: Style::Path
                                )
                            }.into_any();

                            element! {
                                MapItem(name, value)
                            }
                        }))
                    }
                }
                Entry(
                    name: "Environment variables",
                    no_children: environment.vars.is_empty(),
                ) {
                    Map {
                        #(environment.vars.into_iter().map(|(name, value)| {
                            let is_path = value.contains('/') || value.contains('\\');

                            let name = element! {
                                StyledText(
                                    content: name,
                                    style: Style::Property
                                )
                            }.into_any();

                            let value = element! {
                                StyledText(
                                    content: value,
                                    style: if is_path {
                                        Style::Path
                                    } else {
                                        Style::MutedLight
                                    }
                                )
                            }.into_any();

                            element! {
                                MapItem(name, value)
                            }
                        }))
                    }
                }
            }
        }
    })?;

    Ok(None)
}
