mod utils;

use proto_core::Manifest;
use utils::*;

#[test]
fn lists_local_versions() {
    let temp = create_temp_dir();

    let mut manifest = Manifest::load(temp.join("tools/node/manifest.json")).unwrap();
    manifest.default_version = Some("19.0.0".into());
    manifest.installed_versions.insert("19.0.0".into());
    manifest.installed_versions.insert("18.0.0".into());
    manifest.installed_versions.insert("17.0.0".into());
    manifest.save().unwrap();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd.arg("list").arg("node").assert();

    let output = output_to_string(&assert.get_output().stdout);

    assert_eq!(output.split('\n').collect::<Vec<_>>().len(), 4); // includes header
}
