mod utils;

use std::fs;
use utils::*;

mod local {
    use super::*;

    #[test]
    fn writes_local_version_file() {
        let temp = create_empty_sandbox();
        let version_file = temp.path().join(".prototools");

        assert!(!version_file.exists());

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("local")
            .arg("node")
            .arg("19.0.0")
            .assert()
            .success();

        assert!(version_file.exists());
        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            "node = \"19.0.0\"\n"
        )
    }

    #[test]
    fn appends_multiple_tools() {
        let temp = create_empty_sandbox();
        let version_file = temp.path().join(".prototools");

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("local")
            .arg("node")
            .arg("19.0.0")
            .assert()
            .success();

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("local").arg("npm").arg("9.0.0").assert().success();

        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            r#"node = "19.0.0"
npm = "9.0.0"
"#
        )
    }

    #[test]
    fn will_overwrite_by_name() {
        let temp = create_empty_sandbox();
        let version_file = temp.path().join(".prototools");

        temp.create_file(
            ".prototools",
            r#"node = "16.0.0"
npm = "9.0.0"
"#,
        );

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("local")
            .arg("node")
            .arg("19.0.0")
            .assert()
            .success();

        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            r#"node = "19.0.0"
npm = "9.0.0"
"#
        )
    }

    #[test]
    fn can_set_aliases() {
        let temp = create_empty_sandbox();
        let version_file = temp.path().join(".prototools");

        assert!(!version_file.exists());

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("local")
            .arg("npm")
            .arg("bundled")
            .assert()
            .success();

        assert!(version_file.exists());
        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            "npm = \"bundled\"\n"
        )
    }
}
