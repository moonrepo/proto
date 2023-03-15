mod utils;

use predicates::prelude::*;
use std::fs;
use utils::*;

#[test]
fn errors_if_not_installed() {
    let temp = create_temp_dir();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd.arg("run").arg("node").arg("19.0.0").assert();

    assert.stderr(predicate::str::contains(
        "This project requires Node.js 19.0.0, but this version has not been installed. Install it with `proto install node 19.0.0`!",
    ));
}

#[test]
fn errors_if_no_version_detected() {
    let temp = create_temp_dir();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd.arg("run").arg("node").assert();

    assert.stderr(predicate::str::contains(
        "Unable to detect an applicable version",
    ));
}

#[test]
fn runs_a_tool() {
    let temp = create_temp_dir();

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("install")
        .arg("node")
        .arg("19.0.0")
        .assert()
        .success();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd
        .arg("run")
        .arg("node")
        .arg("19.0.0")
        .arg("--")
        .arg("--version")
        .assert();

    assert.stdout(predicate::str::contains("19.0.0"));
}

#[test]
fn runs_a_tool_using_version_detection() {
    let temp = create_temp_dir();

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("install")
        .arg("node")
        .arg("19.0.0")
        .assert()
        .success();

    // Arg
    let mut cmd = create_proto_command(temp.path());
    let assert = cmd
        .arg("run")
        .arg("node")
        .arg("19.0.0")
        .arg("--")
        .arg("--version")
        .assert();

    assert.stdout(predicate::str::contains("19.0.0"));

    // Env var
    let mut cmd = create_proto_command(temp.path());
    let assert = cmd
        .env("PROTO_NODE_VERSION", "19.0.0")
        .arg("run")
        .arg("node")
        .arg("--")
        .arg("--version")
        .assert();

    assert.stdout(predicate::str::contains("19.0.0"));

    // Local version
    fs::write(temp.path().join(".prototools"), "node = \"19.0.0\"").unwrap();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd
        .arg("run")
        .arg("node")
        .arg("--")
        .arg("--version")
        .assert();

    assert.stdout(predicate::str::contains("19.0.0"));

    fs::remove_file(temp.path().join(".prototools")).unwrap();

    // Ecosystem
    fs::write(
        temp.path().join("package.json"),
        r#"{ "engines": { "node": "19.0.0" }}"#,
    )
    .unwrap();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd
        .arg("run")
        .arg("node")
        .arg("--")
        .arg("--version")
        .assert();

    assert.stdout(predicate::str::contains("19.0.0"));

    fs::remove_file(temp.path().join("package.json")).unwrap();

    // Global version
    fs::write(
        temp.path().join("tools/node/manifest.json"),
        r#"{ "default_version": "19.0.0" }"#,
    )
    .unwrap();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd
        .arg("run")
        .arg("node")
        .arg("--")
        .arg("--version")
        .assert();

    assert.stdout(predicate::str::contains("19.0.0"));

    fs::remove_file(temp.path().join("tools/node/manifest.json")).unwrap();
}
