#![allow(dead_code)]

use proto_core::{ProtoConfig, ProtoConfigManager};
use proto_shim::get_exe_file_name;
use starbase_sandbox::{assert_cmd, create_command_with_name};
pub use starbase_sandbox::{create_empty_sandbox, Sandbox};
use std::path::{Path, PathBuf};

pub fn load_config<T: AsRef<Path>>(dir: T) -> ProtoConfig {
    let manager = ProtoConfigManager::load(dir, None).unwrap();
    let config = manager.get_merged_config().unwrap();
    config.to_owned()
}

pub fn create_empty_sandbox_with_tools() -> Sandbox {
    let temp = create_empty_sandbox();

    temp.create_file(
        ".prototools",
        r#"
moon-test = "1.0.0"

[plugins]
moon-test = "source:https://raw.githubusercontent.com/moonrepo/moon/master/proto-plugin.toml"
"#,
    );

    temp
}

pub fn create_proto_command<T: AsRef<Path>>(path: T) -> assert_cmd::Command {
    let path = path.as_ref();

    let mut cmd = create_command_with_name(path, "proto");
    cmd.timeout(std::time::Duration::from_secs(240));
    cmd.env("PROTO_HOME", path.join(".proto"));
    cmd.env("PROTO_LOG", "trace");
    cmd.env("PROTO_WASM_LOG", "trace");
    cmd.env("PROTO_TEST_PROFILE", "true");
    cmd.env("RUST_BACKTRACE", "1");
    cmd.env("WASMTIME_BACKTRACE_DETAILS", "1");
    // cmd.env("PROTO_TEST", "true");
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
