mod utils;

use utils::*;

#[test]
fn lists_remote_versions() {
    let temp = create_temp_dir();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd.arg("list-remote").arg("node").assert();

    let output = output_to_string(&assert.get_output().stdout);

    assert!(output.split('\n').collect::<Vec<_>>().len() > 1);
}
