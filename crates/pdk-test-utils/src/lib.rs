mod macros;
mod wrapper;

pub use macros::*;
pub use proto_core as core;
pub use proto_core::{
    Id, ProtoEnvironment, Tool, ToolManifest, ToolsConfig, UnresolvedVersionSpec, UserConfig,
    Version, VersionReq, VersionSpec, Wasm,
};
pub use proto_pdk_api::*;
pub use wrapper::WasmTestWrapper;

use proto_core::inject_default_manifest_config;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{env, fs};

static mut LOGGING: bool = false;

fn find_target_dir(search_dir: PathBuf) -> Option<PathBuf> {
    let mut dir = search_dir;

    loop {
        let next_target = dir.join("target/wasm32-wasi/debug");

        if next_target.exists() {
            return Some(next_target);
        }

        let next_target = dir.join("wasm32-wasi/debug");

        if next_target.exists() {
            return Some(next_target);
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

            extism::set_log_file(wasm_target_dir.join(format!("{wasm_file_name}.log")), None);
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

pub fn create_plugin(id: &str, sandbox: &Path) -> WasmTestWrapper {
    internal_create_plugin(id, sandbox, HashMap::new())
}

#[allow(unused_variables)]
pub fn create_schema_plugin(id: &str, sandbox: &Path, schema: PathBuf) -> WasmTestWrapper {
    #[allow(unused_mut)]
    let mut config = HashMap::new();

    #[cfg(feature = "schema")]
    {
        let schema = fs::read_to_string(schema).unwrap();
        let schema: serde_json::Value = toml::from_str(&schema).unwrap();

        config.insert(
            "schema".to_string(),
            serde_json::to_string(&schema).unwrap(),
        );
    }

    internal_create_plugin(id, sandbox, config)
}

fn internal_create_plugin(
    id: &str,
    sandbox: &Path,
    config: HashMap<String, String>,
) -> WasmTestWrapper {
    let id = Id::new(id).unwrap();
    let proto = ProtoEnvironment::new_testing(sandbox);

    let mut manifest =
        Tool::create_plugin_manifest(&proto, Wasm::file(find_wasm_file(sandbox))).unwrap();

    inject_default_manifest_config(&id, &proto, &mut manifest).unwrap();

    manifest.config.extend(config);

    WasmTestWrapper {
        tool: Tool::load_from_manifest(Id::new(id).unwrap(), proto, manifest).unwrap(),
    }
}
