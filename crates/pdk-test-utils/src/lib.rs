mod macros;
mod wrapper;

pub use proto_core as core;
pub use proto_core::{
    Id, ProtoEnvironment, Tool, ToolManifest, UnresolvedVersionSpec, Version, VersionReq,
    VersionSpec, Wasm,
};
pub use proto_pdk_api::*;
pub use wrapper::WasmTestWrapper;

use proto_core::{inject_default_manifest_config, inject_proto_manifest_config};
use serde::Serialize;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{env, fs};

static mut LOGGING: bool = false;

fn find_target_dir(search_dir: PathBuf) -> Option<PathBuf> {
    let mut dir = search_dir;
    let profiles = ["debug", "release"];

    loop {
        for profile in &profiles {
            let next_target = dir.join("target/wasm32-wasi").join(profile);

            if next_target.exists() {
                return Some(next_target);
            }

            let next_target = dir.join("wasm32-wasi").join(profile);

            if next_target.exists() {
                return Some(next_target);
            }
        }

        match dir.parent() {
            Some(parent) => {
                dir = parent.to_path_buf();
            }
            None => {
                break;
            }
        };
    }

    None
}

pub fn find_wasm_file(sandbox: &Path) -> PathBuf {
    let wasm_file_name = env::var("CARGO_PKG_NAME").expect("Missing CARGO_PKG_NAME!");

    let mut wasm_target_dir = find_target_dir(
        env::var("CARGO_MANIFEST_DIR")
            .expect("Missing CARGO_MANIFEST_DIR!")
            .into(),
    );

    if wasm_target_dir.is_none() {
        if let Ok(dir) = env::var("CARGO_TARGET_DIR") {
            wasm_target_dir = find_target_dir(dir.into());
        }
    }

    let Some(wasm_target_dir) = wasm_target_dir else {
        panic!("Could not find a target directory!");
    };

    unsafe {
        if !LOGGING {
            LOGGING = true;

            let log_file = wasm_target_dir.join("proto-wasm-plugin.log");
            let log_prefix = wasm_file_name.clone();

            let _ = extism::set_log_callback(
                move |line| {
                    let message = format!("[{log_prefix}] {line}");

                    let mut file = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&log_file)
                        .unwrap();

                    file.write_all(message.as_bytes()).unwrap();
                },
                "trace",
            );
        }
    };

    let wasm_file = wasm_target_dir.join(format!("{wasm_file_name}.wasm"));

    if !wasm_file.exists() {
        panic!(
            "WASM file {:?} does not exist. Please build it with `cargo wasi build` before running tests!",
            wasm_file
        );
    }

    // Folders must exists for WASM to compile correctly!
    fs::create_dir_all(sandbox.join(".home")).unwrap();
    fs::create_dir_all(sandbox.join(".proto")).unwrap();

    wasm_file
}

pub fn create_plugin_with_config(
    id: &str,
    sandbox: &Path,
    config: HashMap<String, String>,
) -> WasmTestWrapper {
    let id = Id::new(id).unwrap();
    let proto = ProtoEnvironment::new_testing(sandbox);

    let mut manifest =
        Tool::create_plugin_manifest(&proto, Wasm::file(find_wasm_file(sandbox))).unwrap();

    inject_default_manifest_config(&id, &proto.home, &mut manifest).unwrap();
    inject_proto_manifest_config(&id, &proto, &mut manifest).unwrap();

    manifest.config.extend(config);

    WasmTestWrapper {
        tool: Tool::load_from_manifest(Id::new(id).unwrap(), proto, manifest).unwrap(),
    }
}

pub fn create_plugin(id: &str, sandbox: &Path) -> WasmTestWrapper {
    create_plugin_with_config(id, sandbox, HashMap::new())
}

#[cfg(feature = "schema")]
pub fn create_schema_plugin_with_config(
    id: &str,
    sandbox: &Path,
    schema: PathBuf,
    mut config: HashMap<String, String>,
) -> WasmTestWrapper {
    let schema = fs::read_to_string(schema).unwrap();
    let schema: serde_json::Value = toml::from_str(&schema).unwrap();

    config.extend([create_config_entry("schema", schema)]);

    create_plugin_with_config(id, sandbox, config)
}

#[cfg(not(feature = "schema"))]
pub fn create_schema_plugin_with_config(
    id: &str,
    sandbox: &Path,
    _schema: PathBuf,
    config: HashMap<String, String>,
) -> WasmTestWrapper {
    create_plugin_with_config(id, sandbox, config)
}

pub fn create_schema_plugin(id: &str, sandbox: &Path, schema: PathBuf) -> WasmTestWrapper {
    create_schema_plugin_with_config(id, sandbox, schema, HashMap::new())
}

pub fn create_config_entry<T: Serialize>(key: &str, value: T) -> (String, String) {
    (key.into(), serde_json::to_string(&value).unwrap())
}

pub fn map_config_environment(os: HostOS, arch: HostArch) -> (String, String) {
    create_config_entry(
        "proto_environment",
        HostEnvironment {
            arch,
            os,
            ..HostEnvironment::default()
        },
    )
}

pub fn map_config_tool_config<T: Serialize>(value: T) -> (String, String) {
    create_config_entry("proto_tool_config", value)
}

pub fn map_config_tool_id(id: &str) -> (String, String) {
    ("proto_tool_id".into(), id.to_owned())
}
