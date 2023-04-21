mod utils;

use predicates::prelude::*;
use utils::*;

#[test]
fn errors_if_not_installed() {
    let temp = create_temp_dir();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd.arg("bin").arg("npm").arg("9.0.0").assert();

    assert.stderr(predicate::str::contains(
        "Unable to find an executable binary for npm",
    ));
}

#[test]
fn returns_path_if_installed() {
    let temp = create_temp_dir();

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("install")
        .arg("npm")
        .arg("9.0.0")
        .assert()
        .success();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd.arg("bin").arg("npm").arg("9.0.0").assert();

    if cfg!(windows) {
        assert.stdout(predicate::str::contains(
            "tools\\npm\\9.0.0\\bin\\npm-cli.js",
        ));
    } else {
        assert.stdout(predicate::str::contains("tools/npm/9.0.0/bin/npm-cli.js"));
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
    let temp = create_temp_dir_with_tools();

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("install")
        .arg("moon-test")
        .arg("1.0.0")
        .assert()
        .success();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd.arg("bin").arg("moon-test").arg("1.0.0").assert();

    if cfg!(windows) {
        assert.stdout(predicate::str::contains(
            "tools\\moon-test\\1.0.0\\moon.exe",
        ));
    } else {
        assert.stdout(predicate::str::contains("tools/moon-test/1.0.0/moon"));
    }
}
