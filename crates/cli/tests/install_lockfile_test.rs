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
                    cmd.arg("install").arg("protostar").arg("5.0.0");
                })
                .success();

            assert!(lockfile_path.exists());

            let lockfile = ProtoLock::load(lockfile_path).unwrap();

            let record = lockfile.tools.get("protostar").unwrap().first().unwrap();

            assert_eq!(
                record.spec.as_ref().unwrap(),
                &UnresolvedVersionSpec::parse("5.0.0").unwrap()
            );
            assert_eq!(
                record.version.as_ref().unwrap(),
                &VersionSpec::parse("5.0.0").unwrap()
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
                    cmd.arg("install").arg("protostar").arg("5.0.0");
                })
                .success();

            assert!(!lockfile_path.exists());
        }

        #[test]
        fn doesnt_track_the_same_spec_version_twice() {
            let sandbox = create_proto_sandbox("lockfile");

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").arg("5.0.0");
                })
                .success();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").arg("5.0.0");
                })
                .success();

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let records = lockfile.tools.get("protostar").unwrap();

            assert_eq!(records.len(), 1);

            let record = records.first().unwrap();

            assert_record!(record, "5.0.0");
        }

        #[test]
        fn tracks_different_specs_and_versions() {
            let sandbox = create_proto_sandbox("lockfile");

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").arg("^5.0");
                })
                .success();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").arg("5.0.0");
                })
                .success();

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let records = lockfile.tools.get("protostar").unwrap();

            assert_eq!(records.len(), 2);
            assert_record!(records[0], "^5.0", "5.10.15");
            assert_record!(records[1], "5.0.0");
        }

        #[test]
        fn tracks_different_tools() {
            let sandbox = create_proto_sandbox("lockfile");

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").arg("2.4.0");
                })
                .success();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("moonshot").arg("1.2.0");
                })
                .success();

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let alt1 = lockfile.tools.get("protostar").unwrap();

            assert_eq!(alt1.len(), 1);
            assert_record!(alt1[0], "2.4.0");

            let alt2 = lockfile.tools.get("moonshot").unwrap();

            assert_eq!(alt2.len(), 1);
            assert_record!(alt2[0], "1.2.0");
        }

        #[test]
        fn updates_existing_spec_with_higher_version() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(
                ".protolock",
                r#"
[[tools.protostar]]
spec = "^5.10"
version = "5.10.0"
"#,
            );

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("protostar")
                        .arg("^5.10")
                        .arg("--update-lockfile");
                })
                .success();

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let records = lockfile.tools.get("protostar").unwrap();

            assert_eq!(records.len(), 1);
            assert_record!(records[0], "^5.10", "5.10.15");
        }

        #[test]
        fn doesnt_update_existing_spec_with_lower_version() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(
                ".protolock",
                r#"
[[tools.protostar]]
spec = "^5.10"
version = "5.10.100"
"#,
            );

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("protostar")
                        .arg("^5.10")
                        .arg("--update-lockfile");
                })
                .success();

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let records = lockfile.tools.get("protostar").unwrap();

            assert_eq!(records.len(), 1);
            assert_record!(records[0], "^5.10", "5.10.100");
        }

        #[test]
        fn doesnt_update_existing_spec_with_different_backend() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(
                ".protolock",
                r#"
[[tools.protostar]]
backend = "asdf"
spec = "^5.10"
version = "5.10.0"
"#,
            );

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").arg("^5.10");
                })
                .success();

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let records = lockfile.tools.get("protostar").unwrap();

            assert_eq!(records.len(), 2);
            assert_record!(records[0], "^5.10", "5.10.0");
            assert_record!(records[1], "^5.10", "5.10.15");
        }

        #[test]
        fn can_override_locked_record_with_flag() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(
                ".protolock",
                r#"
[[tools.protostar]]
spec = "5.10.0"
version = "5.10.0"
checksum = "sha256:invalid"
"#,
            );

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("protostar")
                        .arg("5.10.0")
                        .arg("--update-lockfile");
                })
                .success();

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let records = lockfile.tools.get("protostar").unwrap();

            assert_eq!(records.len(), 1);
            assert_record!(records[0], "5.10.0");

            let checksum = records[0].checksum.as_ref().unwrap();

            assert_eq!(checksum.algo, ChecksumAlgorithm::Sha256);
            assert_ne!(checksum.hash.as_ref().unwrap(), "invalid");
        }

        #[test]
        fn errors_if_locked_version_is_invalid() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(
                ".protolock",
                r#"
[[tools.protostar]]
spec = "^5.10"
version = "5.10.100"
"#,
            );

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").arg("^5.10");
                })
                .failure();

            assert.stderr(predicate::str::contains("Invalid version"));
        }

        #[test]
        fn errors_for_checksum_mismatch() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(
                ".protolock",
                r#"
[[tools.protostar]]
spec = "5.10.0"
version = "5.10.0"
checksum = "sha256:invalid"
"#,
            );

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").arg("5.10.0");
                })
                .failure();

            assert.stderr(predicate::str::contains("Checksum mismatch"));
        }
    }

    mod resolve_version {
        use super::*;

        #[test]
        fn inherits_version_from_file_with_matching_req() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(
                ".protolock",
                r#"
[[tools.protostar]]
spec = "^5.10"
version = "5.10.10"
"#,
            );

            let assert = sandbox
                .run_bin(|cmd| {
                    // 5.10.15 is latest
                    cmd.arg("install").arg("protostar").arg("^5.10");
                })
                .success();

            assert.stdout(predicate::str::contains(
                "protostar 5.10.10 has been installed",
            ));
        }
    }
}
