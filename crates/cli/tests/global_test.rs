mod utils;

use proto_core::Manifest;
use utils::*;

#[test]
fn updates_manifest_file() {
    let temp = create_temp_dir();
    let manifest_file = temp.join("tools/node/manifest.json");

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
fn can_set_alias_as_default() {
    let temp = create_temp_dir();
    let manifest_file = temp.join("tools/npm/manifest.json");

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
