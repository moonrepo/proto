#![allow(dead_code)]

use proto_core::{ProtoConfig, ProtoConfigManager};
use proto_shim::get_exe_file_name;
use starbase_sandbox::{assert_cmd, Sandbox};
use std::collections::HashMap;
use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};

pub struct ProtoSandbox {
    pub sandbox: Sandbox,
}

impl ProtoSandbox {
    pub fn new(mut sandbox: Sandbox) -> Self {
        apply_settings(&mut sandbox);

        Self { sandbox }
    }
}

impl Deref for ProtoSandbox {
    type Target = Sandbox;

    fn deref(&self) -> &Self::Target {
        &self.sandbox
    }
}

fn apply_settings(sandbox: &mut Sandbox) {
    let root = sandbox.path().to_path_buf();
    let home_dir = sandbox.path().join(".home");
    let proto_dir = sandbox.path().join(".proto");

    // Folders must exist or tests fail!
    fs::create_dir_all(&home_dir).unwrap();
    fs::create_dir_all(&proto_dir).unwrap();

    let mut env = HashMap::new();
    env.insert("RUST_BACKTRACE", "1");
    env.insert("WASMTIME_BACKTRACE_DETAILS", "1");
    env.insert("NO_COLOR", "1");
    env.insert("PROTO_SANDBOX", root.to_str().unwrap());
    env.insert("PROTO_HOME", proto_dir.to_str().unwrap());
    env.insert("PROTO_LOG", "trace");
    env.insert("PROTO_TEST", "true");

    sandbox.settings.bin = "proto".into();
    sandbox.settings.timeout = 240;

    sandbox
        .settings
        .env
        .extend(env.into_iter().map(|(k, v)| (k.to_owned(), v.to_owned())));
}

pub fn create_empty_proto_sandbox() -> ProtoSandbox {
    ProtoSandbox::new(starbase_sandbox::create_empty_sandbox())
}

pub fn create_empty_proto_sandbox_with_tools(ext: &str) -> ProtoSandbox {
    let sandbox = create_empty_proto_sandbox();
    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let schema_path = root_dir
        .join("./tests/fixtures")
        .join(format!("moon-schema.{ext}"));

    sandbox.create_file(
        ".prototools",
        format!(
            r#"
moon-test = "1.0.0"

[plugins]
moon-test = "file://{}"
"#,
            schema_path.display()
        ),
    );

    sandbox
}

pub fn create_proto_sandbox<N: AsRef<str>>(fixture: N) -> ProtoSandbox {
    ProtoSandbox::new(starbase_sandbox::create_sandbox(fixture))
}

pub fn load_config<T: AsRef<Path>>(dir: T) -> ProtoConfig {
    let manager = ProtoConfigManager::load(dir, None, None).unwrap();
    let config = manager.get_merged_config().unwrap();
    config.to_owned()
}

pub fn create_shim_command<T: AsRef<Path>>(path: T, name: &str) -> assert_cmd::Command {
    let mut cmd = assert_cmd::Command::from_std(create_shim_command_std(path, name));
    cmd.timeout(std::time::Duration::from_secs(240));
    cmd
}

pub fn create_shim_command_std<T: AsRef<Path>>(path: T, name: &str) -> std::process::Command {
    let path = path.as_ref();

    let mut cmd = std::process::Command::new(get_shim_path(path, name));
    cmd.env("PROTO_LOG", "trace");
    cmd.env("PROTO_HOME", path.join(".proto"));
    cmd.env("PROTO_NODE_VERSION", "latest"); // For package managers
    cmd.env(format!("PROTO_{}_VERSION", name.to_uppercase()), "latest");
    cmd
}

pub fn get_bin_path<T: AsRef<Path>>(path: T, name: &str) -> PathBuf {
    path.as_ref()
        .join(".proto/bin")
        .join(get_exe_file_name(name))
}

pub fn get_shim_path<T: AsRef<Path>>(path: T, name: &str) -> PathBuf {
    path.as_ref()
        .join(".proto/shims")
        .join(get_exe_file_name(name))
}

pub fn link_bin(input_path: &Path, output_path: &Path) {
    fs::create_dir_all(output_path.parent().unwrap()).unwrap();

    #[cfg(windows)]
    {
        fs::copy(input_path, output_path).unwrap();
    }

    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(input_path, output_path).unwrap();
    }
}
