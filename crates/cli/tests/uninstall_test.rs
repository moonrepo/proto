mod utils;

use starbase_sandbox::predicates::prelude::*;
use utils::*;

mod uninstall {
    use super::*;

    #[test]
    fn doesnt_uninstall_tool_if_doesnt_exist() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("uninstall").arg("node").arg("19.0.0").arg("--yes");
        });

        assert.inner.stdout(predicate::str::contains(
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
                cmd.arg("uninstall").arg("node").arg("19.0.0").arg("--yes");
            })
            .success();

        assert!(!sandbox.path().join(".proto/tools/node/19.0.0").exists());
        assert!(sandbox
            .path()
            .join(".proto/tools/node/manifest.json")
            .exists());
    }

    #[test]
    fn doesnt_uninstall_all_if_doesnt_exist() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("uninstall").arg("node").arg("--yes");
        });

        assert.inner.stdout(predicate::str::contains(
            "Node.js has not been installed locally",
        ));
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
                cmd.arg("uninstall").arg("node").arg("19.0.0").arg("--yes");
            })
            .success();

        assert_eq!(
            std::fs::read_to_string(sandbox.path().join(".prototools")).unwrap(),
            ""
        );
    }

    #[allow(deprecated)]
    #[cfg(not(windows))]
    #[test]
    fn removes_tool_bins() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".proto/tools/node/1.2.3/fake/file", "");
        sandbox.create_file(
            ".proto/tools/node/manifest.json",
            r#"{ "installed_versions": ["1.2.3"] }"#,
        );
        sandbox.create_file(".proto/bin/other", "");

        let bin1 = sandbox.path().join(".proto/bin/node");
        let bin2 = sandbox.path().join(".proto/bin/node-1");
        let bin3 = sandbox.path().join(".proto/bin/node-1.2");
        let src = sandbox.path().join(".proto/tools/node/1.2.3/fake/file");

        std::fs::soft_link(&src, &bin1).unwrap();
        std::fs::soft_link(&src, &bin2).unwrap();
        std::fs::soft_link(&src, &bin3).unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall").arg("--yes").arg("node");
            })
            .success();

        assert!(!bin1.exists());
        assert!(bin1.symlink_metadata().is_err());
        assert!(!bin2.exists());
        assert!(bin2.symlink_metadata().is_err());
        assert!(!bin3.exists());
        assert!(bin3.symlink_metadata().is_err());
    }

    #[test]
    fn removes_tool_shims() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".proto/tools/npm/manifest.json", "{}");
        sandbox.create_file(".proto/shims/npm", "");
        sandbox.create_file(".proto/shims/npm.exe", "");
        sandbox.create_file(".proto/shims/npx", "");
        sandbox.create_file(".proto/shims/npx.exe", "");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall").arg("--yes").arg("npm");
            })
            .success();

        if cfg!(windows) {
            assert!(!sandbox.path().join(".proto/shims/npm.exe").exists());
            assert!(!sandbox.path().join(".proto/shims/npx.exe").exists());
        } else {
            assert!(!sandbox.path().join(".proto/shims/npm").exists());
            assert!(!sandbox.path().join(".proto/shims/npx").exists());
        }
    }
}
