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

mod outdated_lockfile {
    use super::*;

    #[test]
    fn updates_lockfile_records_with_newest() {
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
            .run_bin(|cmd| {
                cmd.arg("outdated").arg("--update").arg("--yes");
            })
            .success();

        // Config was updated to the newest version
        let config = fs::read_to_string(sandbox.path().join(".prototools")).unwrap();

        assert!(predicate::str::contains("4.10.15").eval(&config));

        // Lockfile record was migrated to the new spec
        let lock = ProtoLock::load_from(sandbox.path()).unwrap();
        let records = lock.tools.get(&Id::raw("protostar")).unwrap();

        assert_eq!(records.len(), 1);

        let record = records.first().unwrap();

        assert_eq!(
            record.spec.as_ref().unwrap(),
            &UnresolvedVersionSpec::parse("4.10.15").unwrap()
        );
        assert_eq!(
            record.version.as_ref().unwrap(),
            &VersionSpec::parse("4.10.15").unwrap()
        );

        // Checksum is stale for the new version, so it must be removed
        assert!(record.checksum.is_none());

        // While the platform is preserved
        assert_eq!(record.os.as_ref().unwrap(), &SystemOS::default());
        assert_eq!(record.arch.as_ref().unwrap(), &SystemArch::default());
    }

    #[test]
    fn updates_lockfile_records_with_latest() {
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
            .run_bin(|cmd| {
                cmd.arg("outdated")
                    .arg("--update")
                    .arg("--latest")
                    .arg("--yes");
            })
            .success();

        let config = fs::read_to_string(sandbox.path().join(".prototools")).unwrap();

        assert!(predicate::str::contains("5.10.15").eval(&config));

        let lock = ProtoLock::load_from(sandbox.path()).unwrap();
        let records = lock.tools.get(&Id::raw("protostar")).unwrap();

        assert_eq!(records.len(), 1);

        let record = records.first().unwrap();

        assert_eq!(
            record.spec.as_ref().unwrap(),
            &UnresolvedVersionSpec::parse("5.10.15").unwrap()
        );
        assert_eq!(
            record.version.as_ref().unwrap(),
            &VersionSpec::parse("5.10.15").unwrap()
        );
        assert!(record.checksum.is_none());
    }

    #[test]
    fn updates_env_config_and_its_lockfile_records() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(
            ".prototools",
            r#"protostar = "4.0.0"
protoform = "3.0.0"

[settings]
lockfile = true
builtin-backends = false
builtin-tools = false
"#,
        );
        sandbox.create_file(".prototools.production", r#"protostar = "3.0.0""#);

        let mut lock = ProtoLock::default();
        lock.tools.insert(
            Id::raw("protostar"),
            vec![create_lockfile_record("4.0.0", "4.0.0")],
        );
        lock.tools.insert(
            Id::raw("protoform"),
            vec![create_lockfile_record("3.0.0", "3.0.0")],
        );
        lock.path = sandbox.path().join(".protolock");
        lock.save().unwrap();

        let mut lock = ProtoLock::default();
        lock.tools.insert(
            Id::raw("protostar"),
            vec![create_lockfile_record("3.0.0", "3.0.0")],
        );
        lock.path = sandbox.path().join(".protolock.production");
        lock.save().unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("outdated")
                    .arg("--update")
                    .arg("--yes")
                    .env("PROTO_ENV", "production");
            })
            .success();

        // The env config takes precedence for protostar, so it was updated
        let config = fs::read_to_string(sandbox.path().join(".prototools.production")).unwrap();

        assert!(predicate::str::contains(r#"protostar = "3.10.15""#).eval(&config));

        // While the base config was updated for protoform only
        let config = fs::read_to_string(sandbox.path().join(".prototools")).unwrap();

        assert!(predicate::str::contains(r#"protostar = "4.0.0""#).eval(&config));
        assert!(predicate::str::contains(r#"protoform = "3.10.15""#).eval(&config));

        // Each lockfile was migrated to match its config
        let lock = ProtoLock::load(sandbox.path().join(".protolock.production")).unwrap();
        let records = lock.tools.get(&Id::raw("protostar")).unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].spec.as_ref().unwrap(),
            &UnresolvedVersionSpec::parse("3.10.15").unwrap()
        );
        assert_eq!(
            records[0].version.as_ref().unwrap(),
            &VersionSpec::parse("3.10.15").unwrap()
        );
        assert!(records[0].checksum.is_none());

        let lock = ProtoLock::load_from(sandbox.path()).unwrap();
        let records = lock.tools.get(&Id::raw("protostar")).unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].spec.as_ref().unwrap(),
            &UnresolvedVersionSpec::parse("4.0.0").unwrap()
        );
        assert!(records[0].checksum.is_some());

        let records = lock.tools.get(&Id::raw("protoform")).unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].spec.as_ref().unwrap(),
            &UnresolvedVersionSpec::parse("3.10.15").unwrap()
        );
        assert!(records[0].checksum.is_none());
    }

    #[test]
    fn doesnt_create_lockfile_if_disabled() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(
            ".prototools",
            r#"protostar = "4.0.0"

[settings]
builtin-backends = false
builtin-tools = false
"#,
        );

        sandbox
            .run_bin(|cmd| {
                cmd.arg("outdated").arg("--update").arg("--yes");
            })
            .success();

        let config = fs::read_to_string(sandbox.path().join(".prototools")).unwrap();

        assert!(predicate::str::contains("4.10.15").eval(&config));
        assert!(!sandbox.path().join(".protolock").exists());
    }

    #[test]
    fn shows_locked_column_when_records_exist() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(
            ".prototools",
            r#"protostar = "~4.0"

[settings]
lockfile = true
builtin-backends = false
builtin-tools = false
"#,
        );

        let mut lock = ProtoLock::default();
        lock.tools.insert(
            Id::raw("protostar"),
            vec![create_lockfile_record("~4.0", "4.0.3")],
        );
        lock.path = sandbox.path().join(".protolock");
        lock.save().unwrap();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("outdated");
        });

        let output = assert.output();

        assert!(predicate::str::contains("Locked").eval(&output));
        assert!(predicate::str::contains("4.0.3").eval(&output));
    }

    #[test]
    fn hides_locked_column_when_no_records() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(
            ".prototools",
            r#"protostar = "~4.0"

[settings]
builtin-backends = false
builtin-tools = false
"#,
        );

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("outdated");
        });

        let output = assert.output();

        assert!(predicate::str::contains("protostar").eval(&output));
        assert!(predicate::str::contains("Locked").not().eval(&output));
    }

    #[test]
    fn includes_locked_version_in_json() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(
            ".prototools",
            r#"protostar = "~4.0"

[settings]
lockfile = true
builtin-backends = false
builtin-tools = false
"#,
        );

        let mut lock = ProtoLock::default();
        lock.tools.insert(
            Id::raw("protostar"),
            vec![create_lockfile_record("~4.0", "4.0.3")],
        );
        lock.path = sandbox.path().join(".protolock");
        lock.save().unwrap();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("outdated").arg("--json");
        });

        let output = assert.output();

        assert!(predicate::str::contains("locked_version").eval(&output));
        assert!(predicate::str::contains("4.0.3").eval(&output));
    }
}
