mod utils;

use utils::*;

#[test]
fn installs_node_global() {
    let temp = create_empty_sandbox();
    let mut cmd = create_proto_command(temp.path());

    cmd.arg("install-global")
        .arg("node")
        .arg("typescript")
        .assert();

    assert!(temp.path().join("tools/node/globals/bin/tsc").exists());
}
