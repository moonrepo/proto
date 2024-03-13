use serde::Serialize;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use warpgate_api::{HostArch, HostEnvironment, HostLibc, HostOS, TestEnvironment, VirtualPath};

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

pub struct ConfigBuilder {
    config: HashMap<String, String>,
    sandbox_root: PathBuf,
    sandbox_home_dir: PathBuf,
}

impl ConfigBuilder {
    pub fn new(root: &Path, home_dir: &Path) -> Self {
        Self {
            config: HashMap::new(),
            sandbox_root: root.to_path_buf(),
            sandbox_home_dir: home_dir.to_path_buf(),
        }
    }

    pub fn build(mut self) -> HashMap<String, String> {
        if !self.config.contains_key("host_environment") {
            self.host(HostOS::from_env(), HostArch::from_env());
        }

        if !self.config.contains_key("test_environment") {
            self.test_environment(TestEnvironment {
                ci: env::var("CI").is_ok(),
                sandbox: self.sandbox_root.clone(),
            });
        }

        self.config
    }

    pub fn insert(&mut self, key: &str, value: impl Serialize) -> &mut Self {
        self.config
            .insert(key.to_owned(), serde_json::to_string(&value).unwrap());
        self
    }

    pub fn host(&mut self, os: HostOS, arch: HostArch) -> &mut Self {
        self.host_environment(HostEnvironment {
            arch,
            libc: HostLibc::detect(os),
            os,
            home_dir: VirtualPath::default(),
        })
    }

    pub fn host_environment(&mut self, mut env: HostEnvironment) -> &mut Self {
        if env.home_dir.real_path().is_none() || env.home_dir.virtual_path() == Path::new("") {
            env.home_dir = VirtualPath::WithReal {
                path: PathBuf::from("/userhome"),
                virtual_prefix: PathBuf::from("/userhome"),
                real_prefix: self.sandbox_home_dir.clone(),
            };
        }

        self.insert("host_environment", env)
    }

    pub fn test_environment(&mut self, env: TestEnvironment) -> &mut Self {
        self.insert("test_environment", env)
    }

    pub fn plugin_id(&mut self, id: impl AsRef<str>) -> &mut Self {
        self.insert("plugin_id", id.as_ref())
    }
}
