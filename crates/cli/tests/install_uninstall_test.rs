mod utils;

use predicates::prelude::*;
use std::fs;
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
    cmd.arg("install")
        .arg("node")
        .arg("19.0.0")
        .assert()
        .success();

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

    assert.stderr(predicate::str::contains("Node.js v19.0.0 does not exist!"));
}

#[test]
fn updates_the_manifest_when_installing() {
    let temp = create_temp_dir();
    let manifest_file = temp.join("tools/node/manifest.json");

    // Install
    let mut cmd = create_proto_command(temp.path());
    cmd.arg("install")
        .arg("node")
        .arg("19.0.0")
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(&manifest_file).unwrap(),
        r#"{
  "default_version": "19.0.0",
  "installed_versions": [
    "19.0.0"
  ]
}"#
    );

    // Uninstall
    let mut cmd = create_proto_command(temp.path());
    cmd.arg("uninstall")
        .arg("node")
        .arg("19.0.0")
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(&manifest_file).unwrap(),
        r#"{
  "default_version": null,
  "installed_versions": []
}"#
    );
}

#[test]
fn can_pin_when_installing() {
    let temp = create_temp_dir();
    let manifest_file = temp.join("tools/node/manifest.json");

    fs::create_dir_all(manifest_file.parent().unwrap()).unwrap();
    fs::write(
        &manifest_file,
        r#"{
  "default_version": "18.0.0",
  "installed_versions": [
    "18.0.0"
  ]
}"#,
    )
    .unwrap();

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("install")
        .arg("node")
        .arg("19.0.0")
        .arg("--pin")
        .assert();

    assert_eq!(
        fs::read_to_string(&manifest_file).unwrap(),
        r#"{
  "default_version": "19.0.0",
  "installed_versions": [
    "18.0.0",
    "19.0.0"
  ]
}"#
    );
}

mod node {
    use super::*;

    #[test]
    fn installs_bundled_npm() {
        let temp = create_temp_dir();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd.arg("install").arg("node").arg("19.0.0").assert();

        assert!(temp.join("tools/node/19.0.0").exists());
        assert!(temp.join("tools/npm/8.19.2").exists());

        let output = output_to_string(&assert.get_output().stderr.to_vec());

        assert!(predicate::str::contains("Node.js has been installed at").eval(&output));
        assert!(predicate::str::contains("npm has been installed at").eval(&output));
    }
}
