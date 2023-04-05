mod utils;

use std::fs;
use utils::*;

#[test]
fn removes_existing_alias() {
    let temp = create_temp_dir();
    let manifest_file = temp.join("tools/node/manifest.json");

    fs::create_dir_all(manifest_file.parent().unwrap()).unwrap();
    fs::write(
        &manifest_file,
        r#"{
  "aliases": {
    "example": "19.0.0"
  },
  "default_version": null,
  "installed_versions": []
}"#,
    )
    .unwrap();

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("unalias")
        .arg("node")
        .arg("example")
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(manifest_file).unwrap(),
        r#"{
  "aliases": {},
  "default_version": null,
  "installed_versions": []
}"#
    );
}

#[test]
fn does_nothing_for_unknown_alias() {
    let temp = create_temp_dir();
    let manifest_file = temp.join("tools/node/manifest.json");

    fs::create_dir_all(manifest_file.parent().unwrap()).unwrap();
    fs::write(
        &manifest_file,
        r#"{
  "aliases": {
    "example": "19.0.0"
  },
  "default_version": null,
  "installed_versions": []
}"#,
    )
    .unwrap();

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("unalias")
        .arg("node")
        .arg("unknown")
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(manifest_file).unwrap(),
        r#"{
  "aliases": {
    "example": "19.0.0"
  },
  "default_version": null,
  "installed_versions": []
}"#
    );
}
