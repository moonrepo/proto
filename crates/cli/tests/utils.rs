#![allow(dead_code)]

use proto_core::{ProtoConfig, ProtoConfigManager};
use starbase_sandbox::{assert_cmd, create_command_with_name};
pub use starbase_sandbox::{create_empty_sandbox, output_to_string, Sandbox};
use std::path::Path;

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
    cmd.env("RUST_BACKTRACE", "1");
    cmd.env("WASMTIME_BACKTRACE_DETAILS", "1");
    // cmd.env("PROTO_TEST", "true");
    cmd
}

pub fn create_shim_command<T: AsRef<Path>>(path: T, name: &str) -> assert_cmd::Command {
    let path = path.as_ref();

    let mut cmd = assert_cmd::Command::new(path.join(".proto/shims").join(if cfg!(windows) {
        format!("{name}.cmd")
    } else {
        name.to_owned()
    }));
    cmd.timeout(std::time::Duration::from_secs(240));
    cmd.env("PROTO_HOME", path.join(".proto"));
    cmd.env("PROTO_NODE_VERSION", "latest"); // For package managers
    cmd.env(format!("PROTO_{}_VERSION", name.to_uppercase()), "latest");
    cmd
}
