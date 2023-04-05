mod utils;

use predicates::prelude::*;
use std::fs;
use utils::*;

#[test]
fn updates_manifest_file() {
    let temp = create_temp_dir();
    let manifest_file = temp.join("tools/node/manifest.json");

    assert!(!manifest_file.exists());

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("alias")
        .arg("node")
        .arg("example")
        .arg("19.0.0")
        .assert()
        .success();

    assert!(manifest_file.exists());
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

#[test]
fn can_overwrite_existing_alias() {
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
    cmd.arg("alias")
        .arg("node")
        .arg("example")
        .arg("20.0.0")
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(manifest_file).unwrap(),
        r#"{
  "aliases": {
    "example": "20.0.0"
  },
  "default_version": null,
  "installed_versions": []
}"#
    );
}

#[test]
fn errors_when_aliasing_self() {
    let temp = create_temp_dir();
    let manifest_file = temp.join("tools/node/manifest.json");

    assert!(!manifest_file.exists());

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd
        .arg("alias")
        .arg("node")
        .arg("example")
        .arg("example")
        .assert();

    assert.stderr(predicate::str::contains("Cannot map an alias to itself."));
}
