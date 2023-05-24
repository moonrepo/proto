mod utils;

use starbase_sandbox::predicates::prelude::*;
use utils::*;

#[test]
fn errors_if_not_installed() {
    let temp = create_empty_sandbox();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd.arg("bin").arg("npm").arg("9.0.0").assert();

    assert.stderr(predicate::str::contains(
        "Unable to find an executable binary for npm",
    ));
}

#[test]
fn returns_path_if_installed() {
    let temp = create_empty_sandbox();

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("install")
        .arg("npm")
        .arg("9.0.0")
        .assert()
        .success();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd.arg("bin").arg("npm").arg("9.0.0").assert();

    if cfg!(windows) {
        assert.stdout(predicate::str::contains("tools\\npm\\9.0.0\\bin\\npm.cmd"));
    } else {
        assert.stdout(predicate::str::contains("tools/npm/9.0.0/bin/npm"));
    }

    // With shims
    let mut cmd = create_proto_command(temp.path());
    let assert = cmd
        .arg("bin")
        .arg("npm")
        .arg("9.0.0")
        .arg("--shim")
        .assert();

    if cfg!(windows) {
        assert.stdout(predicate::str::contains("tools\\npm\\9.0.0\\shims\\npm"));
    } else {
        assert.stdout(predicate::str::contains("tools/npm/9.0.0/shims/npm"));
    }
}

#[test]
fn returns_path_for_plugin() {
    let temp = create_empty_sandbox_with_tools();

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("install")
        .arg("moon-test")
        .arg("1.0.0")
        .assert()
        .success();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd.arg("bin").arg("moon-test").arg("1.0.0").assert();

    if cfg!(windows) {
        assert.stdout(predicate::str::ends_with(
            "tools\\moon-test\\1.0.0\\moon-test.exe\n",
        ));
    } else {
        assert.stdout(predicate::str::ends_with(
            "tools/moon-test/1.0.0/moon-test\n",
        ));
    }
}
