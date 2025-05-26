use serde::Serialize;
use starbase_utils::env::{is_ci, path_var};
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
    let wasm_file = format!(
        "{}.wasm",
        env::var("CARGO_PKG_NAME").expect("Missing CARGO_PKG_NAME!")
    );

    for env_var in ["CARGO_MANIFEST_DIR", "CARGO_TARGET_DIR"] {
        if let Some(env_path) = path_var(env_var) {
            if let Some(wasm_path) = traverse_target_dir(env_path, &wasm_file) {
                return wasm_path;
            }
        }
    }

    panic!(
        "WASM file `{}` does not exist. Please build it with `cargo build --target wasm32-wasip1` before running tests!",
        wasm_file
    );
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
                ci: is_ci(),
                sandbox: VirtualPath::Real(self.sandbox_root.clone()),
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
            ci: is_ci(),
            libc: HostLibc::detect(os),
            os,
            home_dir: VirtualPath::default(),
        })
    }

    pub fn host_environment(&mut self, mut env: HostEnvironment) -> &mut Self {
        if env.home_dir.real_path().is_none() || env.home_dir.virtual_path().is_none() {
            env.home_dir = VirtualPath::Virtual {
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
