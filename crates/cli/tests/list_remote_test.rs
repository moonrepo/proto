mod utils;

use utils::*;

#[test]
fn lists_remote_versions() {
    let temp = create_empty_sandbox();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd.arg("list-remote").arg("node").assert();

    let output = output_to_string(&assert.get_output().stdout);

    assert!(output.split('\n').collect::<Vec<_>>().len() > 1);
}

#[test]
fn lists_remote_versions_for_plugin() {
    let temp = create_sandbox_with_tools();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd.arg("list-remote").arg("moon-test").assert();

    let output = output_to_string(&assert.get_output().stdout);

    assert!(output.split('\n').collect::<Vec<_>>().len() > 1);
}
