#![allow(dead_code)]

use proto_core::{ProtoConfig, ProtoConfigManager};
use proto_shim::get_exe_file_name;
use starbase_sandbox::{assert_cmd, create_command_with_name, Sandbox, SandboxSettings};
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
    let mut env = HashMap::new();
    env.insert("RUST_BACKTRACE", "1");
    env.insert("WASMTIME_BACKTRACE_DETAILS", "1");
    env.insert("NO_COLOR", "1");
    env.insert("PROTO_LOG", "trace");
    env.insert("PROTO_TEST", "true");

    sandbox.settings.bin = "moon".into();
    sandbox.settings.timeout = 240;

    sandbox
        .settings
        .env
        .extend(env.into_iter().map(|(k, v)| (k.to_owned(), v.to_owned())));
}

pub fn create_empty_proto_sandbox() -> ProtoSandbox {
    ProtoSandbox::new(starbase_sandbox::create_empty_sandbox())
}

pub fn create_empty_proto_sandbox_with_tools() -> ProtoSandbox {
    let sandbox = create_empty_proto_sandbox();

    sandbox.create_file(
        ".prototools",
        r#"
moon-test = "1.0.0"

[plugins]
moon-test = "https://raw.githubusercontent.com/moonrepo/moon/master/proto-plugin.toml"
"#,
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

#[deprecated]
pub fn create_proto_command<T: AsRef<Path>>(path: T) -> assert_cmd::Command {
    let path = path.as_ref();

    let mut cmd = create_command_with_name(path, "proto", &SandboxSettings::default());
    cmd.timeout(std::time::Duration::from_secs(240));
    cmd.env("PROTO_HOME", path.join(".proto"));
    cmd.env("PROTO_LOG", "trace");
    cmd.env("PROTO_WASM_LOG", "trace");
    cmd.env("PROTO_TEST", "true");
    cmd.env("RUST_BACKTRACE", "1");
    cmd.env("WASMTIME_BACKTRACE_DETAILS", "1");
    // cmd.env("EXTISM_DEBUG", "1");
    // cmd.env("EXTISM_ENABLE_WASI_OUTPUT", "1");
    cmd
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
