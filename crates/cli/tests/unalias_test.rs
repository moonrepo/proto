mod utils;

use proto_core::Manifest;
use rustc_hash::FxHashMap;
use utils::*;

#[test]
fn removes_existing_alias() {
    let temp = create_empty_sandbox();
    let manifest_file = temp.path().join("tools/node/manifest.json");

    let mut manifest = Manifest::load(&manifest_file).unwrap();
    manifest.aliases.insert("example".into(), "19.0.0".into());
    manifest.save().unwrap();

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("unalias")
        .arg("node")
        .arg("example")
        .assert()
        .success();

    let manifest = Manifest::load(&manifest_file).unwrap();

    assert!(manifest.aliases.is_empty());
}

#[test]
fn removes_existing_alias_for_plugin() {
    let temp = create_sandbox_with_tools();
    let manifest_file = temp.path().join("tools/moon-test/manifest.json");

    let mut manifest = Manifest::load(&manifest_file).unwrap();
    manifest.aliases.insert("example".into(), "1.0.0".into());
    manifest.save().unwrap();

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("unalias")
        .arg("moon-test")
        .arg("example")
        .assert()
        .success();

    let manifest = Manifest::load(&manifest_file).unwrap();

    assert!(manifest.aliases.is_empty());
}

#[test]
fn does_nothing_for_unknown_alias() {
    let temp = create_empty_sandbox();
    let manifest_file = temp.path().join("tools/node/manifest.json");

    let mut manifest = Manifest::load(&manifest_file).unwrap();
    manifest.aliases.insert("example".into(), "19.0.0".into());
    manifest.save().unwrap();

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("unalias")
        .arg("node")
        .arg("unknown")
        .assert()
        .success();

    let manifest = Manifest::load(manifest_file).unwrap();

    assert_eq!(
        manifest.aliases,
        FxHashMap::from_iter([("example".into(), "19.0.0".into())])
    );
}
