mod utils;

use utils::*;

#[test]
fn updates_manifest_file() {
    let temp = create_temp_dir();
    let manifest_file = temp.join("tools/node/manifest.json");

    assert!(!manifest_file.exists());

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("global").arg("node").arg("19.0.0").assert();

    assert!(manifest_file.exists());
    assert_eq!(
        std::fs::read_to_string(manifest_file).unwrap(),
        r#"{
  "default_version": "19.0.0",
  "installed_versions": []
}"#
    );
}
