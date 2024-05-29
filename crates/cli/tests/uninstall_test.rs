mod utils;

use starbase_sandbox::predicates::prelude::*;
use utils::*;

mod uninstall {
    use super::*;

    #[test]
    fn doesnt_uninstall_tool_if_doesnt_exist() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd.arg("uninstall").arg("node").arg("19.0.0").assert();

        assert.stderr(predicate::str::contains(
            "Node.js 19.0.0 has not been installed locally",
        ));
    }

    #[test]
    fn uninstalls_by_version() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install").arg("node").arg("19.0.0").assert();

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("uninstall").arg("node").arg("19.0.0").assert();

        assert!(!temp.path().join(".proto/tools/node/19.0.0").exists());
        assert!(temp.path().join(".proto/tools/node/manifest.json").exists());
    }

    #[test]
    fn uninstalls_everything() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install").arg("node").arg("19.0.0").assert();

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install").arg("node").arg("20.0.0").assert();

        assert!(temp.path().join(".proto/tools/node/19.0.0").exists());
        assert!(temp.path().join(".proto/tools/node/20.0.0").exists());

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("uninstall").arg("node").arg("--yes").assert();

        assert!(!temp.path().join(".proto/tools/node").exists());
    }

    #[test]
    fn unpins_from_config() {
        let temp = create_empty_sandbox();
        temp.create_file(".prototools", r#"node = "19.0.0""#);

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install").arg("node").arg("19.0.0").assert();

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("uninstall").arg("node").arg("19.0.0").assert();

        assert_eq!(
            std::fs::read_to_string(temp.path().join(".prototools")).unwrap(),
            ""
        );
    }
}
