mod utils;

use proto_core::Manifest;
use starbase_sandbox::create_empty_sandbox;
use utils::*;

#[test]
fn updates_manifest_file() {
    let temp = create_empty_sandbox();
    let manifest_file = temp.path().join("tools/node/manifest.json");

    assert!(!manifest_file.exists());

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("global")
        .arg("node")
        .arg("19.0.0")
        .assert()
        .success();

    assert!(manifest_file.exists());

    let manifest = Manifest::load(manifest_file).unwrap();

    assert_eq!(manifest.default_version, Some("19.0.0".into()));
}

#[test]
fn updates_manifest_file_for_plugin() {
    let temp = create_sandbox_with_tools();
    let manifest_file = temp.path().join("tools/moon-test/manifest.json");

    assert!(!manifest_file.exists());

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("global")
        .arg("moon-test")
        .arg("1.0.0")
        .assert()
        .success();

    assert!(manifest_file.exists());

    let manifest = Manifest::load(manifest_file).unwrap();

    assert_eq!(manifest.default_version, Some("1.0.0".into()));
}

#[test]
fn can_set_alias_as_default() {
    let temp = create_empty_sandbox();
    let manifest_file = temp.path().join("tools/npm/manifest.json");

    assert!(!manifest_file.exists());

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("global")
        .arg("npm")
        .arg("bundled")
        .assert()
        .success();

    assert!(manifest_file.exists());

    let manifest = Manifest::load(manifest_file).unwrap();

    assert_eq!(manifest.default_version, Some("bundled".into()));
}
