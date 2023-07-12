mod macros;
mod wrapper;

pub use macros::*;
pub use proto_wasm_plugin::WasmPlugin;
pub use wrapper::WasmTestWrapper;

use proto_core::Proto;
use std::env;
use std::path::{Path, PathBuf};

static mut LOGGING: bool = false;

pub fn create_plugin(id: &str, sandbox: &Path) -> WasmTestWrapper {
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

    WasmTestWrapper {
        tool: WasmPlugin::new(Proto::from(sandbox), id.into(), wasm_file).unwrap(),
    }
}
