mod utils;

use proto_core::Manifest;
use rustc_hash::FxHashSet;
use starbase_sandbox::predicates::prelude::*;
use utils::*;

#[test]
fn installs_and_uninstalls_tool() {
    let temp = create_empty_sandbox();
    let tool_dir = temp.path().join("tools/node/19.0.0");

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
    let temp = create_empty_sandbox();

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
    let temp = create_empty_sandbox();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd.arg("uninstall").arg("node").arg("19.0.0").assert();

    assert.stderr(predicate::str::contains("Node.js v19.0.0 does not exist!"));
}

#[test]
fn updates_the_manifest_when_installing() {
    let temp = create_empty_sandbox();
    let manifest_file = temp.path().join("tools/node/manifest.json");

    // Install
    let mut cmd = create_proto_command(temp.path());
    cmd.arg("install")
        .arg("node")
        .arg("19.0.0")
        .assert()
        .success();

    let manifest = Manifest::load(&manifest_file).unwrap();

    assert_eq!(manifest.default_version, Some("19.0.0".into()));
    assert_eq!(
        manifest.installed_versions,
        FxHashSet::from_iter(["19.0.0".into()])
    );
    assert!(manifest.versions.contains_key("19.0.0"));

    // Uninstall
    let mut cmd = create_proto_command(temp.path());
    cmd.arg("uninstall")
        .arg("node")
        .arg("19.0.0")
        .assert()
        .success();

    let manifest = Manifest::load(&manifest_file).unwrap();

    assert_eq!(manifest.default_version, None);
    assert_eq!(manifest.installed_versions, FxHashSet::default());
    assert!(!manifest.versions.contains_key("19.0.0"));
}

#[test]
fn can_pin_when_installing() {
    let temp = create_empty_sandbox();
    let manifest_file = temp.path().join("tools/node/manifest.json");

    let mut manifest = Manifest::load(&manifest_file).unwrap();
    manifest.default_version = Some("18.0.0".into());
    manifest.installed_versions.insert("18.0.0".into());
    manifest.save().unwrap();

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("install")
        .arg("node")
        .arg("19.0.0")
        .arg("--pin")
        .assert();

    let manifest = Manifest::load(&manifest_file).unwrap();

    assert_eq!(manifest.default_version, Some("19.0.0".into()));
    assert_eq!(
        manifest.installed_versions,
        FxHashSet::from_iter(["18.0.0".into(), "19.0.0".into()])
    );
}
