mod utils;

use starbase_sandbox::predicates::prelude::*;
use utils::*;

mod uninstall {
    use super::*;

    #[test]
    fn doesnt_uninstall_tool_if_doesnt_exist() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("uninstall")
                .arg("protostar")
                .arg("1.0.0")
                .arg("--yes");
        });

        assert.inner.stdout(predicate::str::contains(
            "protostar 1.0.0 has not been installed locally",
        ));
    }

    #[test]
    fn uninstalls_by_version() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("1.0.0");
            })
            .success();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall")
                    .arg("protostar")
                    .arg("1.0.0")
                    .arg("--yes");
            })
            .success();

        assert!(!sandbox.path().join(".proto/tools/protostar/1.0.0").exists());
        assert!(
            sandbox
                .path()
                .join(".proto/tools/protostar/manifest.json")
                .exists()
        );
    }

    #[test]
    fn doesnt_uninstall_all_if_doesnt_exist() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("uninstall").arg("protostar").arg("--yes");
        });

        assert.inner.stdout(predicate::str::contains(
            "protostar has not been installed locally",
        ));
    }

    #[test]
    fn uninstalls_everything() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("1.0.0");
            })
            .success();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("2.0.0");
            })
            .success();

        assert!(sandbox.path().join(".proto/tools/protostar/1.0.0").exists());
        assert!(sandbox.path().join(".proto/tools/protostar/2.0.0").exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall").arg("protostar").arg("--yes");
            })
            .success();

        assert!(!sandbox.path().join(".proto/tools/protostar").exists());
    }

    #[test]
    fn unpins_from_config() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".prototools", r#"protostar = "1.0.0""#);

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("1.0.0");
            })
            .success();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall")
                    .arg("protostar")
                    .arg("1.0.0")
                    .arg("--yes");
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
        sandbox.create_file(".proto/tools/protostar/1.2.3/fake/file", "");
        sandbox.create_file(
            ".proto/tools/protostar/manifest.json",
            r#"{ "installed_versions": ["1.2.3"] }"#,
        );
        sandbox.create_file(".proto/bin/other", "");

        let bin1 = sandbox.path().join(".proto/bin/protostar");
        let bin2 = sandbox.path().join(".proto/bin/protostar-1");
        let bin3 = sandbox.path().join(".proto/bin/protostar-1.2");
        let src = sandbox
            .path()
            .join(".proto/tools/protostar/1.2.3/fake/file");

        std::fs::soft_link(&src, &bin1).unwrap();
        std::fs::soft_link(&src, &bin2).unwrap();
        std::fs::soft_link(&src, &bin3).unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall").arg("--yes").arg("protostar");
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
        sandbox.create_file(".proto/tools/protostar/manifest.json", "{}");
        sandbox.create_file(".proto/shims/protostar", "");
        sandbox.create_file(".proto/shims/protostar.exe", "");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall").arg("--yes").arg("protostar");
            })
            .success();

        if cfg!(windows) {
            assert!(!sandbox.path().join(".proto\\shims\\protostar.exe").exists());
        } else {
            assert!(!sandbox.path().join(".proto/shims/protostar").exists());
        }
    }
}
