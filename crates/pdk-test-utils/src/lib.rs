mod macros;
mod wrapper;

pub use macros::*;
use proto_core::Id;
pub use proto_core::{AliasOrVersion, ProtoEnvironment, Tool, VersionType};
pub use proto_pdk_api::*;
pub use wrapper::WasmTestWrapper;

use proto_wasm_plugin::Wasm;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{env, fs};

static mut LOGGING: bool = false;

pub fn find_wasm_file(sandbox: &Path) -> PathBuf {
    let mut wasm_target_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("Missing CARGO_MANIFEST_DIR!"));
    let wasm_file_name = env::var("CARGO_PKG_NAME").expect("Missing CARGO_PKG_NAME!");

    loop {
        let next_target = wasm_target_dir.join("target/wasm32-wasi/debug");

        if next_target.exists() {
            wasm_target_dir = next_target;
            break;
        }

        match wasm_target_dir.parent() {
            Some(parent) => wasm_target_dir = parent.to_path_buf(),
            None => panic!("Could not find target directory!"),
        };
    }

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
    let wasm_file = find_wasm_file(sandbox);

    WasmTestWrapper {
        tool: Tool::load(
            Id::new(id).unwrap(),
            ProtoEnvironment::new_testing(sandbox),
            Wasm::file(wasm_file),
        )
        .unwrap(),
    }
}

pub fn create_schema_plugin(id: &str, sandbox: &Path, schema: PathBuf) -> WasmTestWrapper {
    let wasm_file = find_wasm_file(sandbox);
    let proto = ProtoEnvironment::new_testing(sandbox);

    let mut config = HashMap::new();
    config.insert("schema".to_string(), fs::read_to_string(schema).unwrap());

    let mut manifest = Tool::create_plugin_manifest(&proto, Wasm::file(wasm_file)).unwrap();
    manifest = manifest.with_config(config.into_iter());

    WasmTestWrapper {
        tool: Tool::load_from_manifest(Id::new(id).unwrap(), proto, manifest).unwrap(),
    }
}
