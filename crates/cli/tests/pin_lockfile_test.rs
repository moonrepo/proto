use proto_core::test_utils::*;
use proto_core::{Id, LockRecord, ProtoLock, UnresolvedVersionSpec, VersionSpec};
use proto_pdk_api::Checksum;
use starbase_sandbox::predicates::prelude::*;
use std::fs;
use system_env::{SystemArch, SystemOS};

fn create_lockfile_record(spec: &str, version: &str) -> LockRecord {
    LockRecord {
        spec: Some(UnresolvedVersionSpec::parse(spec).unwrap()),
        version: Some(VersionSpec::parse(version).unwrap()),
        checksum: Some(Checksum::sha256("fake_hash".into())),
        os: Some(SystemOS::default()),
        arch: Some(SystemArch::default()),
        ..Default::default()
    }
}

fn create_sandbox_with_record() -> ProtoSandbox {
    let sandbox = create_empty_proto_sandbox();
    sandbox.create_file(
        ".prototools",
        r#"protostar = "4.0.0"

[settings]
lockfile = true
builtin-backends = false
builtin-tools = false
"#,
    );

    let mut lock = ProtoLock::default();
    lock.tools.insert(
        Id::raw("protostar"),
        vec![create_lockfile_record("4.0.0", "4.0.0")],
    );
    lock.path = sandbox.path().join(".protolock");
    lock.save().unwrap();

    sandbox
}

mod pin_lockfile {
    use super::*;

    #[test]
    fn migrates_records_when_pinning_resolved() {
        let sandbox = create_sandbox_with_record();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin")
                    .arg("protostar")
                    .arg("4.0.6")
                    .arg("--resolve");
            })
            .success();

        let config = fs::read_to_string(sandbox.path().join(".prototools")).unwrap();

        assert!(predicate::str::contains("4.0.6").eval(&config));

        let lock = ProtoLock::load_from(sandbox.path()).unwrap();
        let records = lock.tools.get(&Id::raw("protostar")).unwrap();

        assert_eq!(records.len(), 1);

        let record = records.first().unwrap();

        assert_eq!(
            record.spec.as_ref().unwrap(),
            &UnresolvedVersionSpec::parse("4.0.6").unwrap()
        );
        assert_eq!(
            record.version.as_ref().unwrap(),
            &VersionSpec::parse("4.0.6").unwrap()
        );

        // The version changed, so the checksum is no longer valid
        assert!(record.checksum.is_none());
    }

    #[test]
    fn removes_records_when_pinning_unresolved() {
        let sandbox = create_sandbox_with_record();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin").arg("protostar").arg("~4.0");
            })
            .success();

        let config = fs::read_to_string(sandbox.path().join(".prototools")).unwrap();

        assert!(predicate::str::contains("~4.0").eval(&config));

        // The only record was removed, so the lockfile is deleted
        assert!(!sandbox.path().join(".protolock").exists());
    }

    #[test]
    fn keeps_records_when_pinning_same_spec() {
        let sandbox = create_sandbox_with_record();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin").arg("protostar").arg("4.0.0");
            })
            .success();

        let lock = ProtoLock::load_from(sandbox.path()).unwrap();
        let records = lock.tools.get(&Id::raw("protostar")).unwrap();

        assert_eq!(records.len(), 1);

        let record = records.first().unwrap();

        assert_eq!(
            record.spec.as_ref().unwrap(),
            &UnresolvedVersionSpec::parse("4.0.0").unwrap()
        );
        assert_eq!(
            record.checksum.as_ref().unwrap(),
            &Checksum::sha256("fake_hash".into())
        );
    }

    #[test]
    fn migrates_records_when_installing_with_pin() {
        let sandbox = create_sandbox_with_record();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install")
                    .arg("protostar")
                    .arg("5.0.0")
                    .arg("--pin");
            })
            .success();

        let config = fs::read_to_string(sandbox.path().join(".prototools")).unwrap();

        assert!(predicate::str::contains("5.0.0").eval(&config));

        let lock = ProtoLock::load_from(sandbox.path()).unwrap();
        let records = lock.tools.get(&Id::raw("protostar")).unwrap();

        // The stale 4.0.0 record was merged into the freshly installed record
        assert_eq!(records.len(), 1);

        let record = records.first().unwrap();

        assert_eq!(
            record.spec.as_ref().unwrap(),
            &UnresolvedVersionSpec::parse("5.0.0").unwrap()
        );
        assert_eq!(
            record.version.as_ref().unwrap(),
            &VersionSpec::parse("5.0.0").unwrap()
        );

        // The record from the fresh install must keep its checksum
        let checksum = record.checksum.as_ref().unwrap();

        assert_ne!(checksum, &Checksum::sha256("fake_hash".into()));
    }

    mod env_mode {
        use super::*;

        fn create_sandbox_with_env_record() -> ProtoSandbox {
            let sandbox = create_sandbox_with_record();
            sandbox.create_file(".prototools.production", r#"protostar = "5.0.0""#);

            let mut lock = ProtoLock::default();
            lock.tools.insert(
                Id::raw("protostar"),
                vec![create_lockfile_record("5.0.0", "5.0.0")],
            );
            lock.path = sandbox.path().join(".protolock.production");
            lock.save().unwrap();

            sandbox
        }

        #[test]
        fn migrates_records_in_lockfile_of_modified_config() {
            let sandbox = create_sandbox_with_env_record();

            // Pins to `.prototools`, even though the env config
            // takes precedence for the tool
            sandbox
                .run_bin(|cmd| {
                    cmd.arg("pin")
                        .arg("protostar")
                        .arg("4.0.6")
                        .arg("--resolve")
                        .env("PROTO_ENV", "production");
                })
                .success();

            let config = fs::read_to_string(sandbox.path().join(".prototools")).unwrap();

            assert!(predicate::str::contains("4.0.6").eval(&config));

            let config = fs::read_to_string(sandbox.path().join(".prototools.production")).unwrap();

            assert!(predicate::str::contains("5.0.0").eval(&config));

            // The base lockfile is migrated
            let lock = ProtoLock::load_from(sandbox.path()).unwrap();
            let records = lock.tools.get(&Id::raw("protostar")).unwrap();

            assert_eq!(records.len(), 1);
            assert_eq!(
                records[0].spec.as_ref().unwrap(),
                &UnresolvedVersionSpec::parse("4.0.6").unwrap()
            );
            assert_eq!(
                records[0].version.as_ref().unwrap(),
                &VersionSpec::parse("4.0.6").unwrap()
            );

            // While the env lockfile is untouched
            let lock = ProtoLock::load(sandbox.path().join(".protolock.production")).unwrap();
            let records = lock.tools.get(&Id::raw("protostar")).unwrap();

            assert_eq!(records.len(), 1);
            assert_eq!(
                records[0].spec.as_ref().unwrap(),
                &UnresolvedVersionSpec::parse("5.0.0").unwrap()
            );
        }

        #[test]
        fn removes_records_from_lockfile_of_modified_config() {
            let sandbox = create_sandbox_with_env_record();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("unpin")
                        .arg("protostar")
                        .env("PROTO_ENV", "production");
                })
                .success();

            let config = fs::read_to_string(sandbox.path().join(".prototools")).unwrap();

            assert!(predicate::str::contains("protostar").not().eval(&config));

            // Empty lockfiles are removed
            assert!(!sandbox.path().join(".protolock").exists());

            // While the env lockfile is untouched
            let lock = ProtoLock::load(sandbox.path().join(".protolock.production")).unwrap();

            assert!(lock.tools.contains_key(&Id::raw("protostar")));
        }

        #[test]
        fn tracks_install_record_in_lockfile_of_pinned_config() {
            let sandbox = create_sandbox_with_env_record();

            // The env config takes precedence for the tool, but the
            // version is pinned to `.prototools`, so the install record
            // must be tracked by its lockfile
            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install")
                        .arg("protostar")
                        .arg("5.0.6")
                        .arg("--pin")
                        .env("PROTO_ENV", "production");
                })
                .success();

            let config = fs::read_to_string(sandbox.path().join(".prototools")).unwrap();

            assert!(predicate::str::contains("5.0.6").eval(&config));

            let lock = ProtoLock::load_from(sandbox.path()).unwrap();
            let records = lock.tools.get(&Id::raw("protostar")).unwrap();

            // The stale 4.0.0 record was merged into the freshly installed record
            assert_eq!(records.len(), 1);
            assert_eq!(
                records[0].spec.as_ref().unwrap(),
                &UnresolvedVersionSpec::parse("5.0.6").unwrap()
            );
            assert_eq!(
                records[0].version.as_ref().unwrap(),
                &VersionSpec::parse("5.0.6").unwrap()
            );

            // The record from the fresh install must keep its checksum
            let checksum = records[0].checksum.as_ref().unwrap();

            assert_ne!(checksum, &Checksum::sha256("fake_hash".into()));

            // While the env config and its records are not migrated
            let config = fs::read_to_string(sandbox.path().join(".prototools.production")).unwrap();

            assert!(predicate::str::contains("5.0.0").eval(&config));

            let lock = ProtoLock::load(sandbox.path().join(".protolock.production")).unwrap();
            let records = lock.tools.get(&Id::raw("protostar")).unwrap();

            assert!(records.iter().any(|record| {
                record.spec.as_ref().unwrap() == &UnresolvedVersionSpec::parse("5.0.0").unwrap()
                    && record.checksum.as_ref().unwrap() == &Checksum::sha256("fake_hash".into())
            }));
        }
    }
}
