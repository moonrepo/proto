#![allow(dead_code)]

use starbase_sandbox::{assert_cmd, create_command_with_name};
pub use starbase_sandbox::{create_empty_sandbox, output_to_string, Sandbox};
use std::path::Path;

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
    cmd.timeout(std::time::Duration::from_secs(180));
    cmd.env("PROTO_HOME", path.as_os_str());
    cmd.env("PROTO_LOG", "trace");
    cmd.env("RUST_BACKTRACE", "1");
    cmd.env("WASM_BACKTRACE", "1");
    // cmd.env("PROTO_TEST", "true");
    cmd
}
