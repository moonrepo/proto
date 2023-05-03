mod utils;

use proto_core::Manifest;
use rustc_hash::FxHashMap;
use starbase_sandbox::predicates::prelude::*;
use utils::*;

#[test]
fn updates_manifest_file() {
    let temp = create_empty_sandbox();
    let manifest_file = temp.path().join("tools/node/manifest.json");

    assert!(!manifest_file.exists());

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("alias")
        .arg("node")
        .arg("example")
        .arg("19.0.0")
        .assert()
        .success();

    assert!(manifest_file.exists());

    let manifest = Manifest::load(manifest_file).unwrap();

    assert_eq!(
        manifest.aliases,
        FxHashMap::from_iter([("example".into(), "19.0.0".into())])
    );
}

#[test]
fn updates_manifest_file_for_plugin() {
    let temp = create_empty_sandbox_with_tools();
    let manifest_file = temp.path().join("tools/moon-test/manifest.json");

    assert!(!manifest_file.exists());

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("alias")
        .arg("moon-test")
        .arg("example")
        .arg("1.0.0")
        .assert()
        .success();

    assert!(manifest_file.exists());

    let manifest = Manifest::load(manifest_file).unwrap();

    assert_eq!(
        manifest.aliases,
        FxHashMap::from_iter([("example".into(), "1.0.0".into())])
    );
}

#[test]
fn can_overwrite_existing_alias() {
    let temp = create_empty_sandbox();
    let manifest_file = temp.path().join("tools/node/manifest.json");

    let mut manifest = Manifest::load(&manifest_file).unwrap();
    manifest.aliases.insert("example".into(), "19.0.0".into());
    manifest.save().unwrap();

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("alias")
        .arg("node")
        .arg("example")
        .arg("20.0.0")
        .assert()
        .success();

    let manifest = Manifest::load(&manifest_file).unwrap();

    assert_eq!(
        manifest.aliases,
        FxHashMap::from_iter([("example".into(), "20.0.0".into())])
    );
}

#[test]
fn errors_when_using_version() {
    let temp = create_empty_sandbox();
    let manifest_file = temp.path().join("tools/node/manifest.json");

    assert!(!manifest_file.exists());

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd
        .arg("alias")
        .arg("node")
        .arg("1.2.3")
        .arg("4.5.6")
        .assert();

    assert.stderr(predicate::str::contains(
        "Versions cannot be aliases. Use alphanumeric words instead.",
    ));
}

#[test]
fn errors_when_aliasing_self() {
    let temp = create_empty_sandbox();
    let manifest_file = temp.path().join("tools/node/manifest.json");

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
