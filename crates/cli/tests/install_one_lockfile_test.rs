use proto_core::test_utils::*;
use proto_core::{ProtoLock, UnresolvedVersionSpec, VersionSpec};
use proto_pdk_api::ChecksumAlgorithm;
use starbase_sandbox::predicates::prelude::*;
use system_env::{SystemArch, SystemOS};

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

mod install_one_lockfile {
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

            assert_eq!(record.os.as_ref().unwrap(), &SystemOS::default());
            assert_eq!(record.arch.as_ref().unwrap(), &SystemArch::default());
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

            // Sorted!
            assert_record!(records[0], "5.0.0");
            assert_record!(records[1], "^5.0", "5.10.15");
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
                    cmd.arg("install").arg("moonstone").arg("1.2.0");
                })
                .success();

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let alt1 = lockfile.tools.get("protostar").unwrap();

            assert_eq!(alt1.len(), 1);
            assert_record!(alt1[0], "2.4.0");

            let alt2 = lockfile.tools.get("moonstone").unwrap();

            assert_eq!(alt2.len(), 1);
            assert_record!(alt2[0], "1.2.0");
        }

        #[test]
        fn can_disable_os_arch_tracking() {
            let sandbox = create_proto_sandbox("lockfile");

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protoform").arg("^5.0");
                })
                .success();

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let records = lockfile.tools.get("protoform").unwrap();

            assert_eq!(records.len(), 1);
            assert_record!(records[0], "^5.0", "5.10.15");
            assert!(records[0].os.is_none());
            assert!(records[0].arch.is_none());
        }

        #[test]
        fn updates_existing_spec_with_higher_version() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(
                ".protolock",
                format!(
                    r#"
[[tools.protostar]]
os = "{}"
arch = "{}"
spec = "^5.10"
version = "5.10.0"
"#,
                    SystemOS::default(),
                    SystemArch::default()
                ),
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
        fn updates_existing_spec_with_missing_os_arch() {
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
            let record = &records[0];

            assert_eq!(records.len(), 1);
            assert_record!(record, "^5.10", "5.10.15");
            assert_eq!(record.os.as_ref().unwrap(), &SystemOS::default());
            assert_eq!(record.arch.as_ref().unwrap(), &SystemArch::default());
        }

        #[test]
        fn doesnt_update_existing_spec_with_lower_version() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(
                ".protolock",
                format!(
                    r#"
[[tools.protostar]]
os = "{}"
arch = "{}"
spec = "^5.10"
version = "5.10.100"
"#,
                    SystemOS::default(),
                    SystemArch::default()
                ),
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
                format!(
                    r#"
[[tools.protostar]]
os = "{}"
arch = "{}"
backend = "asdf"
spec = "^5.10"
version = "5.10.0"
"#,
                    SystemOS::default(),
                    SystemArch::default()
                ),
            );

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").arg("^5.10");
                })
                .success();

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let records = lockfile.tools.get("protostar").unwrap();

            assert_eq!(records.len(), 2);
            assert_record!(records[0], "^5.10", "5.10.15");
            assert_record!(records[1], "^5.10", "5.10.0");
        }

        #[test]
        fn doesnt_update_existing_spec_with_different_os() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(
                ".protolock",
                format!(
                    r#"
[[tools.protostar]]
os = "{}"
arch = "{}"
spec = "^5.10"
version = "5.10.0"
"#,
                    SystemOS::Android,
                    SystemArch::default()
                ),
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
        fn doesnt_update_existing_spec_with_different_arch() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(
                ".protolock",
                format!(
                    r#"
[[tools.protostar]]
os = "{}"
arch = "{}"
spec = "^5.10"
version = "5.10.0"
"#,
                    SystemOS::default(),
                    SystemArch::Mips64
                ),
            );

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").arg("^5.10");
                })
                .success();

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let records = lockfile.tools.get("protostar").unwrap();

            assert_eq!(records.len(), 2);
            assert_record!(records[0], "^5.10", "5.10.15");
            assert_record!(records[1], "^5.10", "5.10.0");
        }

        #[test]
        fn can_override_locked_record_with_flag() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(
                ".protolock",
                format!(
                    r#"
[[tools.protostar]]
os = "{}"
arch = "{}"
spec = "5.10.0"
version = "5.10.0"
checksum = "sha256:invalid"
"#,
                    SystemOS::default(),
                    SystemArch::default()
                ),
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
                format!(
                    r#"
[[tools.protostar]]
os = "{}"
arch = "{}"
spec = "^5.10"
version = "5.10.100"
"#,
                    SystemOS::default(),
                    SystemArch::default()
                ),
            );

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").arg("^5.10");
                })
                .failure();

            assert.stderr(predicate::str::contains("Failed"));
        }

        #[test]
        fn errors_for_checksum_mismatch() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(
                ".protolock",
                format!(
                    r#"
[[tools.protostar]]
os = "{}"
arch = "{}"
spec = "5.10.0"
version = "5.10.0"
checksum = "sha256:invalid"
"#,
                    SystemOS::default(),
                    SystemArch::default()
                ),
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
                format!(
                    r#"
[[tools.protostar]]
os = "{}"
arch = "{}"
spec = "^5.10"
version = "5.10.10"
"#,
                    SystemOS::default(),
                    SystemArch::default()
                ),
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

    mod env_mode {
        use super::*;

        fn create_locked_record(spec: &str, version: &str) -> String {
            format!(
                r#"
[[tools.protostar]]
os = "{}"
arch = "{}"
spec = "{spec}"
version = "{version}"
"#,
                SystemOS::default(),
                SystemArch::default()
            )
        }

        #[test]
        fn locks_env_config_to_env_lockfile() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(".prototools.production", r#"protostar = "5.0.0""#);

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("protostar")
                        .env("PROTO_ENV", "production");
                })
                .success();

            // The env config defines the version, so the
            // record is written to its lockfile
            let lockfile = ProtoLock::load(sandbox.path().join(".protolock.production")).unwrap();
            let records = lockfile.tools.get("protostar").unwrap();

            assert_eq!(records.len(), 1);
            assert_record!(records[0], "5.0.0");

            assert!(!sandbox.path().join(".protolock").exists());
        }

        #[test]
        fn locks_adhoc_install_to_base_config() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(".prototools.production", "");

            // Not pinned in any config, so the base config
            // owns the record, even when in env mode
            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("protostar")
                        .arg("5.0.0")
                        .env("PROTO_ENV", "production");
                })
                .success();

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let records = lockfile.tools.get("protostar").unwrap();

            assert_eq!(records.len(), 1);
            assert_record!(records[0], "5.0.0");

            assert!(!sandbox.path().join(".protolock.production").exists());
        }

        #[test]
        fn locks_adhoc_install_to_env_config_when_base_config_missing() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                ".prototools.production",
                r#"
[settings]
lockfile = true
"#,
            );

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("protostar")
                        .arg("5.0.0")
                        .env("PROTO_ENV", "production");
                })
                .success();

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock.production")).unwrap();
            let records = lockfile.tools.get("protostar").unwrap();

            assert_eq!(records.len(), 1);
            assert_record!(records[0], "5.0.0");

            assert!(!sandbox.path().join(".protolock").exists());
        }

        #[test]
        fn inherits_version_from_env_lockfile() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(
                ".prototools",
                r#"
protostar = "^5.10"

[settings]
unstable-lockfile = true
"#,
            );
            sandbox.create_file(".protolock", create_locked_record("^5.10", "5.10.5"));
            sandbox.create_file(".prototools.production", r#"protostar = "^5.10""#);
            sandbox.create_file(
                ".protolock.production",
                create_locked_record("^5.10", "5.10.10"),
            );

            // Resolves from the env lockfile
            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("protostar")
                        .env("PROTO_ENV", "production");
                })
                .success();

            assert.stdout(predicate::str::contains(
                "protostar 5.10.10 has been installed",
            ));

            // And the base lockfile without an env
            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar");
                })
                .success();

            assert.stdout(predicate::str::contains(
                "protostar 5.10.5 has been installed",
            ));

            // Neither lockfile was modified
            let lockfile = ProtoLock::load(sandbox.path().join(".protolock.production")).unwrap();
            let records = lockfile.tools.get("protostar").unwrap();

            assert_eq!(records.len(), 1);
            assert_record!(records[0], "^5.10", "5.10.10");

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let records = lockfile.tools.get("protostar").unwrap();

            assert_eq!(records.len(), 1);
            assert_record!(records[0], "^5.10", "5.10.5");
        }

        #[test]
        fn doesnt_create_env_lockfile_if_disabled_in_env_config() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(
                ".prototools.production",
                r#"
protostar = "5.0.0"

[settings]
lockfile = false
"#,
            );

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("protostar")
                        .env("PROTO_ENV", "production");
                })
                .success();

            assert!(!sandbox.path().join(".protolock.production").exists());
            assert!(!sandbox.path().join(".protolock").exists());
        }
    }
}
