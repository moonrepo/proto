#![allow(dead_code)]

use starbase_sandbox::create_command_with_name;
pub use starbase_sandbox::{assert_cmd, create_empty_sandbox, Sandbox};
use std::path::Path;

pub fn output_to_string(data: &[u8]) -> String {
    String::from_utf8(data.to_vec()).unwrap_or_default()
}

pub fn create_sandbox_with_tools() -> Sandbox {
    let temp = create_empty_sandbox();

    temp.create_file(
        ".prototools",
        r#"
moon-test = "1.0.0"

[plugins]
moon-test = "schema:https://raw.githubusercontent.com/moonrepo/moon/master/proto-plugin.toml"
"#,
    );

    temp
}

pub fn create_proto_command<T: AsRef<Path>>(path: T) -> assert_cmd::Command {
    let path = path.as_ref();

    let mut cmd = create_command_with_name(path, "proto");
    cmd.timeout(std::time::Duration::from_secs(180));
    cmd.env("PROTO_ROOT", path.as_os_str());
    cmd.env("PROTO_LOG", "trace");
    cmd.env("PROTO_TEST", "true");
    cmd
}
