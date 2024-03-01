mod macros;
mod wrapper;

pub use proto_core as core;
pub use proto_core::{
    Id, ProtoEnvironment, Tool, ToolManifest, UnresolvedVersionSpec, Version, VersionReq,
    VersionSpec,
};
pub use proto_pdk_api::*;
pub use warpgate::Wasm;
pub use wrapper::WasmTestWrapper;

use proto_core::{get_home_dir, inject_proto_manifest_config};
use serde::Serialize;
use std::collections::HashMap;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use warpgate::{inject_default_manifest_config, test_utils};

pub fn find_wasm_file(sandbox: &Path) -> PathBuf {
    let wasm_file = test_utils::find_wasm_file();

    // Folders must exist for WASM to compile correctly!
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

    let wasm_file = find_wasm_file(sandbox);
    let mut log_file = wasm_file.clone();
    log_file.set_extension("log");

    let mut manifest = Tool::create_plugin_manifest(&proto, Wasm::file(wasm_file)).unwrap();

    inject_default_manifest_config(&id, &proto.home, &mut manifest).unwrap();
    inject_proto_manifest_config(&id, &proto, &mut manifest).unwrap();
    manifest.config.extend(config);

    let test_config = map_config_test_environment(sandbox);
    manifest.config.insert(test_config.0, test_config.1);

    // Remove the file otherwise it keeps growing
    if log_file.exists() {
        let _ = fs::remove_file(&log_file);
    }

    // TODO redo
    if env::var("CI").is_err() {
        let _ = extism::set_log_callback(
            move |line| {
                let mut file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log_file)
                    .unwrap();

                file.write_all(line.as_bytes()).unwrap();
            },
            "debug",
        );
    }

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
    map_config_environment_with_home(os, arch, get_home_dir().unwrap())
}

pub fn map_config_environment_with_home(
    os: HostOS,
    arch: HostArch,
    home_dir: impl AsRef<Path>,
) -> (String, String) {
    create_config_entry(
        "host_environment",
        HostEnvironment {
            arch,
            os,
            home_dir: VirtualPath::WithReal {
                path: PathBuf::from("/userhome"),
                virtual_prefix: PathBuf::from("/userhome"),
                real_prefix: home_dir.as_ref().to_path_buf(),
            },
        },
    )
}

pub fn map_config_test_environment(sandbox: impl AsRef<Path>) -> (String, String) {
    create_config_entry(
        "test_environment",
        TestEnvironment {
            ci: env::var("CI").is_ok(),
            sandbox: sandbox.as_ref().to_path_buf(),
        },
    )
}

pub fn map_config_tool_config<T: Serialize>(value: T) -> (String, String) {
    create_config_entry("proto_tool_config", value)
}

pub fn map_config_id(id: &str) -> (String, String) {
    ("plugin_id".into(), id.to_owned())
}
