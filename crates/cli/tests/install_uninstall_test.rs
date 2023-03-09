mod utils;

use predicates::prelude::*;
use utils::*;

#[test]
fn installs_and_uninstalls_tool() {
    let temp = create_temp_dir();
    let tool_dir = temp.join("tools/node/19.0.0");

    assert!(!tool_dir.exists());

    // Install
    let mut cmd = create_proto_command(temp.path());
    let assert = cmd.arg("install").arg("node").arg("19.0.0").assert();

    assert!(tool_dir.exists());

    assert.stderr(predicate::str::contains("Node.js has been installed at"));

    // Uninstall
    let mut cmd = create_proto_command(temp.path());
    let assert = cmd.arg("uninstall").arg("node").arg("19.0.0").assert();

    assert!(!tool_dir.exists());

    assert.stderr(predicate::str::contains(
        "Node.js v19.0.0 has been uninstalled!",
    ));
}

#[test]
fn doesnt_install_tool_if_exists() {
    let temp = create_temp_dir();

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("install").arg("node").arg("19.0.0").assert();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd.arg("install").arg("node").arg("19.0.0").assert();

    assert.stderr(predicate::str::contains(
        "Node.js has already been installed",
    ));
}

#[test]
fn doesnt_uninstall_tool_if_doesnt_exist() {
    let temp = create_temp_dir();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd.arg("uninstall").arg("node").arg("19.0.0").assert();

    assert.stderr(predicate::str::contains("Node.js v19.0.0  does not exist!"));
}
