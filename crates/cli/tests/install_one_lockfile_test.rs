use proto_core::test_utils::*;
use proto_core::{LockRecord, ProtoLock, UnresolvedVersionSpec, VersionSpec};
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
        fn can_override_env_locked_record_with_flag() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(".prototools.production", r#"protostar = "5.10.0""#);

            let invalid_record = format!(
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
            );

            sandbox.create_file(".protolock", &invalid_record);
            sandbox.create_file(".protolock.production", &invalid_record);

            // Fails verification against the env lockfile
            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("protostar")
                        .env("PROTO_ENV", "production");
                })
                .failure()
                .stderr(predicate::str::contains("Checksum mismatch"));

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("protostar")
                        .arg("--update-lockfile")
                        .env("PROTO_ENV", "production");
                })
                .success();

            // The env lockfile record was replaced
            let lockfile = ProtoLock::load(sandbox.path().join(".protolock.production")).unwrap();
            let records = lockfile.tools.get("protostar").unwrap();

            assert_eq!(records.len(), 1);
            assert_record!(records[0], "5.10.0");
            assert_ne!(
                records[0].checksum.as_ref().unwrap().hash.as_ref().unwrap(),
                "invalid"
            );

            // While the base lockfile is untouched
            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let records = lockfile.tools.get("protostar").unwrap();

            assert_eq!(records.len(), 1);
            assert_eq!(
                records[0].checksum.as_ref().unwrap().hash.as_ref().unwrap(),
                "invalid"
            );
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

    mod orphan_pruning {
        use super::*;

        fn create_prune_sandbox() -> ProtoSandbox {
            let sandbox = create_empty_proto_sandbox();

            sandbox.create_file(
                ".prototools",
                r#"
protostar = "1"
protoform = "2.1"

[settings]
unstable-lockfile = true
builtin-backends = false
builtin-tools = false
"#,
            );

            sandbox.create_file(
                ".protolock",
                format!(
                    r#"
[[tools.protoform]]
os = "{os}"
arch = "{arch}"
spec = "3.0.0"
version = "3.0.0"

[[tools.moonstone]]
os = "{os}"
arch = "{arch}"
spec = "5.0.0"
version = "5.0.0"

[[tools.protostar]]
os = "{os}"
arch = "{arch}"
spec = "1"
version = "1.10.15"

[[tools.protostar]]
os = "{os}"
arch = "{arch}"
spec = "^4.5"
version = "4.5.15"

[[tools.protostar]]
os = "{os}"
arch = "{arch}"
spec = "5.10.0"
version = "5.10.0"
"#,
                    os = SystemOS::default(),
                    arch = SystemArch::default()
                ),
            );

            sandbox
        }

        fn record_specs(records: &[LockRecord]) -> Vec<String> {
            let mut specs = records
                .iter()
                .map(|record| record.spec.as_ref().unwrap().to_string())
                .collect::<Vec<_>>();
            specs.sort();
            specs
        }

        // Parse the expected values, as specs are normalized when
        // parsed, e.g. "1" becomes "~1"
        fn parsed_specs(list: &[&str]) -> Vec<String> {
            let mut specs = list
                .iter()
                .map(|spec| UnresolvedVersionSpec::parse(spec).unwrap().to_string())
                .collect::<Vec<_>>();
            specs.sort();
            specs
        }

        #[test]
        fn prunes_orphaned_records_across_all_tools() {
            let sandbox = create_prune_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar");
                })
                .success();

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();

            // Specs that no longer match the config were pruned
            let protostar = lockfile.tools.get("protostar").unwrap();

            assert_eq!(record_specs(protostar), parsed_specs(&["1"]));
            assert_record!(protostar[0], "1", "1.10.15");

            // Stale records for other configured tools are also pruned
            assert!(!lockfile.tools.contains_key("protoform"));

            // Unconfigured tools are ad-hoc installs and are kept
            let moonstone = lockfile.tools.get("moonstone").unwrap();

            assert_eq!(record_specs(moonstone), parsed_specs(&["5.0.0"]));
        }

        #[test]
        fn prunes_record_for_unpinned_explicit_version() {
            let sandbox = create_prune_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").arg("5.10.0");
                })
                .success();

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let protostar = lockfile.tools.get("protostar").unwrap();

            // Pruning runs after the install against the config as it stands,
            // and the config still only defines "1". The record just written
            // for 5.10.0 would never be resolved from, so it goes too
            assert_eq!(record_specs(protostar), parsed_specs(&["1"]));
        }

        #[test]
        fn keeps_record_for_explicit_version_when_pinned() {
            let sandbox = create_prune_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("protostar")
                        .arg("5.10.0")
                        .arg("--pin");
                })
                .success();

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let protostar = lockfile.tools.get("protostar").unwrap();

            // Pinning rewrites the config before pruning reloads it, so the
            // record for the installed spec is configured and kept, while the
            // spec it replaced is now orphaned and pruned
            assert_eq!(record_specs(protostar), parsed_specs(&["5.10.0"]));

            let installed = protostar
                .iter()
                .find(|record| {
                    record.spec.as_ref().unwrap()
                        == &UnresolvedVersionSpec::parse("5.10.0").unwrap()
                })
                .unwrap();

            assert!(installed.checksum.is_some());
        }

        #[test]
        fn doesnt_prune_when_internal() {
            let sandbox = create_prune_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("protostar")
                        .arg("5.10.0")
                        .arg("--internal");
                })
                .success();

            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();

            // Internal installs are triggered by other flows, like `proto run`,
            // which shouldn't be reconciling the lockfile
            assert_eq!(
                record_specs(lockfile.tools.get("protostar").unwrap()),
                parsed_specs(&["1", "^4.5", "5.10.0"])
            );
            assert!(lockfile.tools.contains_key("protoform"));
        }

        #[test]
        fn prunes_the_active_env_lockfile_only() {
            let sandbox = create_empty_proto_sandbox();

            sandbox.create_file(
                ".prototools",
                r#"
protostar = "1"

[settings]
unstable-lockfile = true
builtin-backends = false
builtin-tools = false
"#,
            );

            // Each env config owns a lockfile of the same scope
            sandbox.create_file(".prototools.dev", r#"moonstone = "5.0.0""#);
            sandbox.create_file(".prototools.prod", r#"moonstone = "4.0.0""#);

            sandbox.create_file(
                ".protolock",
                format!(
                    r#"
[[tools.protostar]]
os = "{os}"
arch = "{arch}"
spec = "1"
version = "1.10.15"

[[tools.protostar]]
os = "{os}"
arch = "{arch}"
spec = "3.0.0"
version = "3.0.0"
"#,
                    os = SystemOS::default(),
                    arch = SystemArch::default()
                ),
            );

            let env_records = |spec: &str| {
                format!(
                    r#"
[[tools.moonstone]]
os = "{os}"
arch = "{arch}"
spec = "{spec}"
version = "{spec}"

[[tools.moonstone]]
os = "{os}"
arch = "{arch}"
spec = "1.0.0"
version = "1.0.0"
"#,
                    os = SystemOS::default(),
                    arch = SystemArch::default()
                )
            };

            sandbox.create_file(".protolock.dev", env_records("5.0.0"));
            sandbox.create_file(".protolock.prod", env_records("4.0.0"));

            // Run in the dev environment, so the prod config is never loaded
            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").env("PROTO_ENV", "dev");
                })
                .success();

            // The base lockfile is pruned against the base config
            let base_lock = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();

            assert_eq!(
                record_specs(base_lock.tools.get("protostar").unwrap()),
                parsed_specs(&["1"])
            );

            // And the active env lockfile against its own env config
            let dev_lock = ProtoLock::load(sandbox.path().join(".protolock.dev")).unwrap();

            assert_eq!(
                record_specs(dev_lock.tools.get("moonstone").unwrap()),
                parsed_specs(&["5.0.0"])
            );

            // While an inactive environment is left untouched, as its
            // lockfile is never loaded by any flow
            let prod_lock = ProtoLock::load(sandbox.path().join(".protolock.prod")).unwrap();

            assert_eq!(
                record_specs(prod_lock.tools.get("moonstone").unwrap()),
                parsed_specs(&["1.0.0", "4.0.0"])
            );
        }
    }

    mod immutable {
        use super::*;

        fn create_record(spec: &str, version: &str) -> String {
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
        fn installs_from_locked_record_without_modifying_lockfile() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(".protolock", create_record("5.0.0", "5.0.0"));

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("protostar")
                        .arg("5.0.0")
                        .arg("--immutable-lockfile");
                })
                .success();

            assert.stdout(predicate::str::contains(
                "protostar 5.0.0 has been installed",
            ));

            // The lockfile is authoritative and read-only, so no checksum
            // was written back for the installed version
            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let records = lockfile.tools.get("protostar").unwrap();

            assert_eq!(records.len(), 1);
            assert_record!(records[0], "5.0.0");
            assert!(records[0].checksum.is_none());
        }

        #[test]
        fn inherits_locked_version_from_range() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(".protolock", create_record("^5.10", "5.10.10"));

            // 5.10.15 is the latest, but the immutable record pins 5.10.10
            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("protostar")
                        .arg("^5.10")
                        .arg("--immutable-lockfile");
                })
                .success();

            assert.stdout(predicate::str::contains(
                "protostar 5.10.10 has been installed",
            ));
        }

        #[test]
        fn errors_when_record_is_missing() {
            let sandbox = create_proto_sandbox("lockfile");

            // The lockfile is enabled but has no record for the tool
            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("protostar")
                        .arg("5.0.0")
                        .arg("--immutable-lockfile");
                })
                .failure();

            assert.stderr(predicate::str::contains("Lockfile is immutable"));

            // Nothing was written to the lockfile
            assert!(!sandbox.path().join(".protolock").exists());
        }

        #[test]
        fn errors_when_requested_version_isnt_locked() {
            let sandbox = create_proto_sandbox("lockfile");
            sandbox.create_file(".protolock", create_record("5.0.0", "5.0.0"));

            // The lockfile only has 5.0.0, so installing 5.10.0 would
            // require adding a new record
            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("protostar")
                        .arg("5.10.0")
                        .arg("--immutable-lockfile");
                })
                .failure();

            assert.stderr(predicate::str::contains("Lockfile is immutable"));

            // And the existing record was left untouched
            let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
            let records = lockfile.tools.get("protostar").unwrap();

            assert_eq!(records.len(), 1);
            assert_record!(records[0], "5.0.0");
        }

        #[test]
        fn errors_when_combined_with_update_lockfile() {
            let sandbox = create_proto_sandbox("lockfile");

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("protostar")
                        .arg("5.0.0")
                        .arg("--immutable-lockfile")
                        .arg("--update-lockfile");
                })
                .failure()
                .stderr(predicate::str::contains("cannot be used with"));
        }

        #[test]
        fn errors_when_combined_with_pin() {
            let sandbox = create_proto_sandbox("lockfile");

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("protostar")
                        .arg("5.0.0")
                        .arg("--immutable-lockfile")
                        .arg("--pin");
                })
                .failure()
                .stderr(predicate::str::contains("cannot be used with"));
        }
    }
}
