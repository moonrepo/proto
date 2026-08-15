use proto_core::test_utils::*;
use proto_core::{Id, LockRecord, ProtoLock, UnresolvedVersionSpec, VersionSpec};
use starbase_sandbox::predicates::prelude::*;
use system_env::{SystemArch, SystemOS};

fn create_lockfile_record(spec: &str, version: &str) -> LockRecord {
    LockRecord {
        spec: Some(UnresolvedVersionSpec::parse(spec).unwrap()),
        version: Some(VersionSpec::parse(version).unwrap()),
        os: Some(SystemOS::default()),
        arch: Some(SystemArch::default()),
        ..Default::default()
    }
}

mod status_lockfile {
    use super::*;

    #[test]
    fn shows_locked_column_when_resolved_from_lockfile() {
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
            cmd.arg("status");
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
            cmd.arg("status");
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
            cmd.arg("status").arg("--json");
        });

        let output = assert.output();

        assert!(predicate::str::contains("locked_version").eval(&output));
        assert!(predicate::str::contains("4.0.3").eval(&output));
    }

    #[test]
    fn resolves_locked_version_from_env_lockfile() {
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
        sandbox.create_file(".prototools.production", r#"protostar = "~4.0""#);

        let mut lock = ProtoLock::default();
        lock.tools.insert(
            Id::raw("protostar"),
            vec![create_lockfile_record("~4.0", "4.0.3")],
        );
        lock.path = sandbox.path().join(".protolock");
        lock.save().unwrap();

        let mut lock = ProtoLock::default();
        lock.tools.insert(
            Id::raw("protostar"),
            vec![create_lockfile_record("~4.0", "4.0.7")],
        );
        lock.path = sandbox.path().join(".protolock.production");
        lock.save().unwrap();

        // The env config takes precedence, so its lockfile is used
        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("status")
                .arg("--json")
                .env("PROTO_ENV", "production");
        });

        let output = assert.output();

        assert!(predicate::str::contains(r#""locked_version": "4.0.7""#).eval(&output));
        assert!(predicate::str::contains(".prototools.production").eval(&output));

        // Otherwise the base lockfile is used
        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("status").arg("--json");
        });

        let output = assert.output();

        assert!(predicate::str::contains(r#""locked_version": "4.0.3""#).eval(&output));
    }
}
