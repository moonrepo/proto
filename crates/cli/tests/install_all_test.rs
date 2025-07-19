mod utils;

use proto_core::{LockRecord, ToolManifest, VersionSpec};
use proto_pdk_api::Checksum;
use starbase_sandbox::predicates::prelude::*;
use std::path::Path;
use utils::*;

mod install_all {
    use super::*;

    #[test]
    fn installs_all_tools() {
        let sandbox = create_empty_proto_sandbox();
        let protostar_path = sandbox.path().join(".proto/tools/protostar/1.0.0");
        let moonstone_path = sandbox.path().join(".proto/tools/moonstone/2.0.0");
        let moonbase_path = sandbox.path().join(".proto/tools/moonbase/3.0.0");

        sandbox.create_file(
            ".prototools",
            r#"protostar = "1.0.0"
moonstone = "2.0.0"
moonbase = "3.0.0"
    "#,
        );

        assert!(!protostar_path.exists());
        assert!(!moonstone_path.exists());
        assert!(!moonbase_path.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install"); // use
            })
            .success();

        assert!(protostar_path.exists());
        assert!(moonstone_path.exists());
        assert!(moonbase_path.exists());
    }

    #[test]
    fn installs_tool_via_detection() {
        let sandbox = create_empty_proto_sandbox();
        let protostar_path = sandbox.path().join(".proto/tools/protostar/1.0.0");

        sandbox.create_file(".protostarrc", "1.0.0");

        assert!(!protostar_path.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("use"); // install
            })
            .success();

        assert!(protostar_path.exists());
    }

    #[test]
    fn doesnt_install_global_tools() {
        let sandbox = create_empty_proto_sandbox();
        let protostar_path = sandbox.path().join(".proto/tools/protostar/1.0.0");
        let moonstone_path = sandbox.path().join(".proto/tools/moonstone/3.0.0");

        sandbox.create_file(".prototools", r#"protostar = "1.0.0""#);
        sandbox.create_file(".proto/.prototools", r#"moonstone = "3.0.0""#);

        assert!(!protostar_path.exists());
        assert!(!moonstone_path.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("use");
            })
            .success();

        assert!(protostar_path.exists());
        assert!(!moonstone_path.exists());
    }

    #[test]
    fn installs_global_tools_when_included() {
        let sandbox = create_empty_proto_sandbox();
        let protostar_path = sandbox.path().join(".proto/tools/protostar/1.0.0");
        let moonstone_path = sandbox.path().join(".proto/tools/moonstone/3.0.0");

        sandbox.create_file(".prototools", r#"protostar = "1.0.0""#);
        sandbox.create_file(".proto/.prototools", r#"moonstone = "3.0.0""#);

        assert!(!protostar_path.exists());
        assert!(!moonstone_path.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install")
                    .arg("--config-mode")
                    .arg("upwards-global");
            })
            .success();

        assert!(protostar_path.exists());
        assert!(moonstone_path.exists());
    }

    #[test]
    fn creates_log_for_each_failed_tool() {
        let sandbox = create_empty_proto_sandbox();

        sandbox.create_file(
            ".prototools",
            r#"protostar = "invalid"
protoform = "invalid"
moonstone = "latest"
    "#,
        );

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install"); // use
            })
            .failure();

        assert!(sandbox.path().join("proto-protostar-install.log").exists());
        assert!(sandbox.path().join("proto-protoform-install.log").exists());
        assert!(!sandbox.path().join("proto-moonstone-install.log").exists());
    }

    mod requirements {
        use super::*;

        #[test]
        fn errors_if_reqs_not_met() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", r#"moonbase = "2.0.0""#);

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("install");
                })
                .failure();

            assert.stderr(predicate::str::contains(
                "moonbase requires moonstone to function correctly",
            ));
        }

        #[test]
        fn passes_if_reqs_met() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                ".prototools",
                r#"moonbase = "1.0.0"
moonstone = "2.0.0"
        "#,
            );

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("install");
                })
                .success();

            assert.stdout(
                predicate::str::contains("moonstone 2.0.0 installed")
                    .and(predicate::str::contains("moonbase 1.0.0 installed")),
            );
        }
    }

    mod manifest_lockfile {
        use super::*;

        #[test]
        fn creates_all_lockfiles() {
            let sandbox = create_empty_proto_sandbox();
            let protostar_path = sandbox.path().join(".proto/tools/protostar/1.0.0");
            let moonstone_path = sandbox.path().join(".proto/tools/moonstone/2.0.0");
            let moonbase_path = sandbox.path().join(".proto/tools/moonbase/3.0.0");

            sandbox.create_file(
                ".prototools",
                r#"protostar = "1.0.0"
moonstone = "2.0.0"
moonbase = "3.0.0"
    "#,
            );

            assert!(!protostar_path.exists());
            assert!(!moonstone_path.exists());
            assert!(!moonbase_path.exists());

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install"); // use
                })
                .success();

            assert!(protostar_path.exists());
            assert!(moonstone_path.exists());
            assert!(moonbase_path.exists());

            fn get_lock(dir: &Path, spec: VersionSpec) -> LockRecord {
                let mut manifest = ToolManifest::load_from(dir).unwrap();
                manifest.versions.remove(&spec).unwrap().lock.unwrap()
            }

            assert_eq!(
                get_lock(
                    protostar_path.parent().unwrap(),
                    VersionSpec::parse("1.0.0").unwrap()
                ),
                LockRecord {
                    // spec: Some(UnresolvedVersionSpec::parse("1.0.0").unwrap()),
                    // version: Some(VersionSpec::parse("1.0.0").unwrap()),
                    checksum: Some(Checksum::sha256(
                        "92521fc3cbd964bdc9f584a991b89fddaa5754ed1cc96d6d42445338669c1305".into()
                    )),
                    ..Default::default()
                }
            );

            assert_eq!(
                get_lock(
                    moonstone_path.parent().unwrap(),
                    VersionSpec::parse("2.0.0").unwrap()
                ),
                LockRecord {
                    // spec: Some(UnresolvedVersionSpec::parse("2.0.0").unwrap()),
                    // version: Some(VersionSpec::parse("2.0.0").unwrap()),
                    checksum: Some(Checksum::sha256(
                        "f22abd6773ab232869321ad4b1e47ac0c908febf4f3a2bd10c8066140f741261".into()
                    )),
                    ..Default::default()
                }
            );

            assert_eq!(
                get_lock(
                    moonbase_path.parent().unwrap(),
                    VersionSpec::parse("3.0.0").unwrap()
                ),
                LockRecord {
                    // spec: Some(UnresolvedVersionSpec::parse("3.0.0").unwrap()),
                    // version: Some(VersionSpec::parse("3.0.0").unwrap()),
                    checksum: Some(Checksum::sha256(
                        "c9163ff21f1f2b0390dc48bdda47179718f772f507a7cebceca59ce1a7129029".into()
                    )),
                    ..Default::default()
                }
            );
        }
    }
}
