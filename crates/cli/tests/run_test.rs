mod utils;

use predicates::prelude::*;
use proto_core::Manifest;
use std::fs;
use utils::*;

#[test]
fn errors_if_not_installed() {
    let temp = create_temp_dir();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd.arg("run").arg("node").arg("19.0.0").assert();

    assert.stderr(predicate::str::contains(
        "This project requires Node.js 19.0.0",
    ));
}

#[test]
fn errors_if_no_version_detected() {
    let temp = create_temp_dir();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd.arg("run").arg("node").assert();

    assert.stderr(predicate::str::contains(
        "Failed to detect an applicable version",
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

#[test]
fn updates_last_used_at() {
    let temp = create_temp_dir();
    let manifest_file = temp.join("tools/node/manifest.json");

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("install")
        .arg("node")
        .arg("19.0.0")
        .assert()
        .success();

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("run")
        .arg("node")
        .arg("19.0.0")
        .arg("--")
        .arg("--version")
        .assert();

    let manifest = Manifest::load(&manifest_file).unwrap();
    let last_used_at = manifest.versions.get("19.0.0").unwrap().last_used_at;

    assert!(last_used_at.is_some());

    // Run again and make sure timestamps update
    let mut cmd = create_proto_command(temp.path());
    cmd.arg("run")
        .arg("node")
        .arg("19.0.0")
        .arg("--")
        .arg("--version")
        .assert();

    let manifest = Manifest::load(&manifest_file).unwrap();

    assert_ne!(
        last_used_at,
        manifest.versions.get("19.0.0").unwrap().last_used_at
    );
}

#[test]
fn auto_installs_if_missing() {
    let temp = create_temp_dir();

    std::fs::write(temp.path().join("config.toml"), "auto-install = true").unwrap();

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
fn doesnt_auto_install_if_false() {
    let temp = create_temp_dir();

    std::fs::write(temp.path().join("config.toml"), "auto-install = false").unwrap();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd.arg("run").arg("node").arg("19.0.0").assert();

    assert.stderr(predicate::str::contains(
        "This project requires Node.js 19.0.0",
    ));
}

#[test]
fn runs_a_plugin() {
    let temp = create_temp_dir_with_tools();

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("install")
        .arg("moon-test")
        .arg("1.0.0")
        .assert()
        .success();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd
        .arg("run")
        .arg("moon-test")
        .arg("1.0.0")
        .arg("--")
        .arg("--version")
        .assert();

    assert.stdout(predicate::str::contains("1.0.0"));
}

#[test]
fn errors_if_plugin_not_configured() {
    let temp = create_temp_dir();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd.arg("run").arg("plugin-name").arg("1.0.0").assert();

    assert.stderr(predicate::str::contains(
        "plugin-name is not a built-in tool and has not been configured",
    ));
}
