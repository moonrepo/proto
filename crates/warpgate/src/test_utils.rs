//! Utilities for locating and testing WASM plugins.

use serde::Serialize;
use starbase_utils::envx::{is_ci, path_var};
use starbase_utils::fs;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use warpgate_api::{HostArch, HostEnvironment, HostLibc, HostOS, TestEnvironment, VirtualPath};

fn traverse_target_dir<T: AsRef<Path>, F: AsRef<str>>(
    search_dir: T,
    search_file: F,
) -> Option<PathBuf> {
    let mut dir = search_dir.as_ref();
    let file = search_file.as_ref();
    let profiles = ["release", "debug"];
    let targets = ["wasm32-wasip1", "wasm32-wasi"];

    loop {
        for profile in &profiles {
            for target in &targets {
                let mut next_target = dir.join("target").join(target).join(profile);

                if !file.is_empty() {
                    next_target = next_target.join(file);
                }

                if next_target.exists() {
                    return Some(next_target);
                }

                let mut next_target = dir.join(target).join(profile);

                if !file.is_empty() {
                    next_target = next_target.join(file);
                }

                if next_target.exists() {
                    return Some(next_target);
                }
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

/// Find the WASM compiled target directory.
pub fn find_target_dir<T: AsRef<Path>>(search_dir: T) -> Option<PathBuf> {
    traverse_target_dir(search_dir, "")
}

/// Find an applicable WASM file to run tests with. Will attempt to find
/// the file based on the Cargo package name and target directories.
pub fn find_wasm_file() -> PathBuf {
    let name = env::var("CARGO_PKG_NAME").expect("Missing CARGO_PKG_NAME!");

    find_wasm_file_with_name(&name).unwrap_or_else(|| {
        panic!("WASM file `{name}.wasm` does not exist. Please build it with `cargo build --target wasm32-wasip1` before running tests!")
    })
}

/// Find an applicable WASM file with the provided name to run tests with.
/// Will attempt to find the file based on the Cargo package name and target directories.
pub fn find_wasm_file_with_name(name: &str) -> Option<PathBuf> {
    let wasm_file = format!("{name}.wasm");

    for env_var in [
        "WARPGATE_PLUGINS_DIR",
        "CARGO_MANIFEST_DIR",
        "CARGO_TARGET_DIR",
    ] {
        if let Some(env_path) = path_var(env_var)
            && let Some(wasm_path) = traverse_target_dir(env_path, &wasm_file)
        {
            return Some(wasm_path);
        }
    }

    if let Some(wasm_path) = traverse_target_dir(env::current_dir().unwrap(), &wasm_file) {
        return Some(wasm_path);
    }

    None
}

/// Enable logging for the provided WASM file by extracting any `tracing` logs
/// fired from within WASM and writing them to a local file in the current directory.
pub fn enable_wasm_logging(wasm_file: &Path) {
    use std::io::Write;

    let log_file = std::env::current_dir().unwrap().join(
        wasm_file
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .replace(".wasm", ".log"),
    );

    // Remove the file otherwise it keeps growing
    if log_file.exists() {
        let _ = fs::remove_file(&log_file);
    }

    let _ = extism::set_log_callback(
        move |line| {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_file)
                .unwrap();

            file.write_all(line.as_bytes()).unwrap();
        },
        "trace",
    );
}

/// A builder for the plugin manifest config map, used within tests.
#[derive(Debug)]
pub struct ConfigBuilder {
    config: HashMap<String, String>,
    sandbox_root: PathBuf,
    sandbox_home_dir: PathBuf,
}

impl ConfigBuilder {
    /// Create a new builder with the provided sandbox root and home directory.
    pub fn new(root: &Path, home_dir: &Path) -> Self {
        Self {
            config: HashMap::new(),
            sandbox_root: root.to_path_buf(),
            sandbox_home_dir: home_dir.to_path_buf(),
        }
    }

    /// Build and return the config map, injecting default `host_environment`
    /// and `test_environment` values when not defined.
    pub fn build(mut self) -> HashMap<String, String> {
        if !self.config.contains_key("host_environment") {
            self.host(HostOS::from_env(), HostArch::from_env());
        }

        if !self.config.contains_key("test_environment") {
            self.test_environment(TestEnvironment {
                ci: is_ci(),
                sandbox: VirtualPath::new(self.sandbox_root.clone()),
            });
        }

        // TODO virtual paths?

        self.config
    }

    /// Insert a config setting with the provided key, serializing the value to JSON.
    pub fn insert(&mut self, key: &str, value: impl Serialize) -> &mut Self {
        self.config
            .insert(key.to_owned(), serde_json::to_string(&value).unwrap());
        self
    }

    /// Set the `host_environment` config setting with the provided
    /// operating system and architecture.
    pub fn host(&mut self, os: HostOS, arch: HostArch) -> &mut Self {
        self.host_environment(HostEnvironment {
            arch,
            ci: is_ci(),
            libc: HostLibc::detect(os),
            os,
            home_dir: VirtualPath::default(),
        })
    }

    /// Set the `host_environment` config setting with default values,
    /// which can be customized with the provided function.
    pub fn host_with(&mut self, mut op: impl FnMut(&mut HostEnvironment)) -> &mut Self {
        let os = HostOS::default();
        let mut host = HostEnvironment {
            arch: HostArch::default(),
            ci: is_ci(),
            libc: HostLibc::detect(os),
            os,
            home_dir: VirtualPath::default(),
        };

        op(&mut host);

        self.host_environment(host)
    }

    /// Set the `host_environment` config setting with the provided [`HostEnvironment`].
    /// If the home directory is empty, it will default to `/userhome`.
    pub fn host_environment(&mut self, mut env: HostEnvironment) -> &mut Self {
        if env.home_dir.is_empty() {
            env.home_dir = VirtualPath::new("/userhome");
        }

        self.insert("host_environment", env)
    }

    /// Set the `test_environment` config setting with the provided [`TestEnvironment`].
    pub fn test_environment(&mut self, env: TestEnvironment) -> &mut Self {
        self.insert("test_environment", env)
    }

    /// Set the `plugin_id` config setting.
    pub fn plugin_id(&mut self, id: impl AsRef<str>) -> &mut Self {
        self.config.insert("plugin_id".into(), id.as_ref().into());
        self
    }
}
