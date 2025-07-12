mod utils;

use proto_core::{ProtoLock, UnresolvedVersionSpec, VersionSpec};
use proto_pdk_api::ChecksumAlgorithm;
use starbase_sandbox::predicates::prelude::*;
use utils::*;

macro_rules! assert_record {
    ($var:expr, $spec:literal) => {
        assert_record!($var, $spec, $spec);
    };
    ($var:expr, $spec:literal, $ver:literal) => {
        assert_eq!(
            $var.spec.as_ref().unwrap(),
            &UnresolvedVersionSpec::parse($spec).unwrap()
        );
        assert_eq!(
            $var.version.as_ref().unwrap(),
            &VersionSpec::parse($ver).unwrap()
        );
    };
}

mod install_lockfile {
    use super::*;

    mod create_or_update {
        use super::*;

        #[test]
        fn creates_lockfile_if_enabled() {
            let sandbox = create_proto_sandbox("lockfile");
            let lockfile_path = sandbox.path().join(".protolock");

            assert!(!lockfile_path.exists());

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("node").arg("18.12.0");
                })
                .success();

            assert!(lockfile_path.exists());

            let lockfile = ProtoLock::load(lockfile_path).unwrap();

            let record = lockfile.tools.get("node").unwrap().first().unwrap();

            assert_eq!(
                record.spec.as_ref().unwrap(),
                &UnresolvedVersionSpec::parse("18.12.0").unwrap()
            );
            assert_eq!(
                record.version.as_ref().unwrap(),
                &VersionSpec::parse("18.12.0").unwrap()
            );
            assert_eq!(
                record.checksum.as_ref().unwrap().algo,
                ChecksumAlgorithm::Sha256
            );
            assert!(record.backend.is_none());
            assert!(record.source.is_none());
        }

        #[test]
        fn doesnt_create_lockfile_if_disabled() {
            let sandbox = create_empty_proto_sandbox();
            let lockfile_path = sandbox.path().join(".protolock");

            assert!(!lockfile_path.exists());

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("node").arg("18.12.0");
                })
                .success();

            assert!(!lockfile_path.exists());
        }

        #[test]
        fn doesnt_track_the_same_spec_version_twice() {
            let sandbox = create_proto_sandbox("lockfile");

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("node").arg("18.12.0");
                })
                .success();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("node").arg("18.12.0");
                })
                .success();

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let records = lockfile.tools.get("node").unwrap();

            assert_eq!(records.len(), 1);

            let record = records.first().unwrap();

            assert_record!(record, "18.12.0");
        }

        #[test]
        fn tracks_different_specs_and_versions() {
            let sandbox = create_proto_sandbox("lockfile");

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("node").arg("^18.12");
                })
                .success();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("node").arg("18.12.0");
                })
                .success();

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let records = lockfile.tools.get("node").unwrap();

            assert_eq!(records.len(), 2);
            assert_record!(records[0], "^18.12", "18.20.8");
            assert_record!(records[1], "18.12.0");
        }

        #[test]
        fn tracks_different_tools() {
            let sandbox = create_proto_sandbox("lockfile");

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("deno").arg("2.4.0");
                })
                .success();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("bun").arg("1.2.0");
                })
                .success();

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let deno = lockfile.tools.get("deno").unwrap();

            assert_eq!(deno.len(), 1);
            assert_record!(deno[0], "2.4.0");

            let bun = lockfile.tools.get("bun").unwrap();

            assert_eq!(bun.len(), 1);
            assert_record!(bun[0], "1.2.0");
        }

        #[test]
        fn updates_existing_spec_with_higher_version() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(
                ".protolock",
                r#"
[[tools.node]]
spec = "^18.20"
version = "18.20.0"
"#,
            );

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("node")
                        .arg("^18.20")
                        .arg("--update-lockfile");
                })
                .success();

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let records = lockfile.tools.get("node").unwrap();

            assert_eq!(records.len(), 1);
            assert_record!(records[0], "^18.20", "18.20.8");
        }

        #[test]
        fn doesnt_update_existing_spec_with_lower_version() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(
                ".protolock",
                r#"
[[tools.node]]
spec = "^18.20"
version = "18.20.100"
"#,
            );

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("node")
                        .arg("^18.20")
                        .arg("--update-lockfile");
                })
                .success();

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let records = lockfile.tools.get("node").unwrap();

            assert_eq!(records.len(), 1);
            assert_record!(records[0], "^18.20", "18.20.100");
        }

        #[test]
        fn errors_if_locked_version_is_invalid() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(
                ".protolock",
                r#"
[[tools.node]]
spec = "^18.20"
version = "18.20.100"
"#,
            );

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("node").arg("^18.20");
                })
                .failure();

            assert.stderr(predicate::str::contains("Unable to download file"));
        }

        #[test]
        fn errors_for_checksum_mismatch() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(
                ".protolock",
                r#"
[[tools.node]]
spec = "18.20.8"
version = "18.20.8"
checksum = "sha256:invalid"
"#,
            );

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("node").arg("18.20.8");
                })
                .failure();

            assert.stderr(predicate::str::contains("Checksum mismatch"));
        }

        #[test]
        fn can_override_locked_record_with_flag() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(
                ".protolock",
                r#"
[[tools.node]]
spec = "18.20.8"
version = "18.20.8"
checksum = "sha256:invalid"
"#,
            );

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("node")
                        .arg("18.20.8")
                        .arg("--update-lockfile");
                })
                .success();

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let records = lockfile.tools.get("node").unwrap();

            assert_eq!(records.len(), 1);
            assert_record!(records[0], "18.20.8");

            let checksum = records[0].checksum.as_ref().unwrap();

            assert_eq!(checksum.algo, ChecksumAlgorithm::Sha256);
            assert_ne!(checksum.hash.as_ref().unwrap(), "invalid");
        }
    }
}
