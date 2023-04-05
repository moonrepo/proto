mod utils;

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
    assert_eq!(
        std::fs::read_to_string(manifest_file).unwrap(),
        r#"{
  "aliases": {},
  "default_version": "19.0.0",
  "installed_versions": []
}"#
    );
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
    assert_eq!(
        std::fs::read_to_string(manifest_file).unwrap(),
        r#"{
  "aliases": {},
  "default_version": "bundled",
  "installed_versions": []
}"#
    );
}
