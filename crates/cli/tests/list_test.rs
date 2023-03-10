mod utils;

use std::fs;
use utils::*;

#[test]
fn lists_local_versions() {
    let temp = create_temp_dir();

    fs::create_dir_all(temp.join("tools/node")).unwrap();
    fs::write(
        temp.join("tools/node/manifest.json"),
        r#"{
  "default_version": "19.0.0",
  "installed_versions": [
    "19.0.0",
    "18.0.0",
    "17.0.0"
  ]
}"#,
    )
    .unwrap();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd.arg("list").arg("node").assert();

    let output = output_to_string(&assert.get_output().stdout);

    assert_eq!(output.split('\n').collect::<Vec<_>>().len(), 4); // includes header
}
