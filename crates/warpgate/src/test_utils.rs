use std::env;
use std::path::{Path, PathBuf};

pub fn find_target_dir<T: AsRef<Path>>(search_dir: T) -> Option<PathBuf> {
    let mut dir = search_dir.as_ref();
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
                dir = parent;
            }
            None => {
                break;
            }
        };
    }

    None
}

pub fn find_wasm_file() -> PathBuf {
    let wasm_file_name = env::var("CARGO_PKG_NAME").expect("Missing CARGO_PKG_NAME!");

    let mut wasm_target_dir =
        find_target_dir(env::var("CARGO_MANIFEST_DIR").expect("Missing CARGO_MANIFEST_DIR!"));

    if wasm_target_dir.is_none() {
        if let Ok(dir) = env::var("CARGO_TARGET_DIR") {
            wasm_target_dir = find_target_dir(dir);
        }
    }

    let Some(wasm_target_dir) = wasm_target_dir else {
        panic!("Could not find a target directory!");
    };

    let wasm_file = wasm_target_dir.join(format!("{wasm_file_name}.wasm"));

    if !wasm_file.exists() {
        panic!(
            "WASM file {} does not exist. Please build it with `cargo wasi build` before running tests!",
            wasm_file.display()
        );
    }

    wasm_file
}
