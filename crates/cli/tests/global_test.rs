mod utils;

use utils::*;

#[test]
fn writes_global_version_file() {
    let temp = create_temp_dir();
    let version_file = temp.join("tools/node/version");

    assert!(!version_file.exists());

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("global").arg("node").arg("19.0.0").assert();

    assert!(version_file.exists());
    assert_eq!(std::fs::read_to_string(version_file).unwrap(), "19.0.0")
}
