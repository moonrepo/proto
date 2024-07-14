mod utils;

use starbase_sandbox::predicates::prelude::*;
use utils::*;

mod uninstall {
    use super::*;

    #[test]
    fn doesnt_uninstall_tool_if_doesnt_exist() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("uninstall").arg("node").arg("19.0.0");
        });

        assert.inner.stderr(predicate::str::contains(
            "Node.js 19.0.0 has not been installed locally",
        ));
    }

    #[test]
    fn uninstalls_by_version() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("node").arg("19.0.0");
            })
            .success();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall").arg("node").arg("19.0.0");
            })
            .success();

        assert!(!sandbox.path().join(".proto/tools/node/19.0.0").exists());
        assert!(sandbox
            .path()
            .join(".proto/tools/node/manifest.json")
            .exists());
    }

    #[test]
    fn uninstalls_everything() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("node").arg("19.0.0");
            })
            .success();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("node").arg("20.0.0");
            })
            .success();

        assert!(sandbox.path().join(".proto/tools/node/19.0.0").exists());
        assert!(sandbox.path().join(".proto/tools/node/20.0.0").exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall").arg("node").arg("--yes");
            })
            .success();

        assert!(!sandbox.path().join(".proto/tools/node").exists());
    }

    #[test]
    fn unpins_from_config() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".prototools", r#"node = "19.0.0""#);

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("node").arg("19.0.0");
            })
            .success();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall").arg("node").arg("19.0.0");
            })
            .success();

        assert_eq!(
            std::fs::read_to_string(sandbox.path().join(".prototools")).unwrap(),
            ""
        );
    }
}
