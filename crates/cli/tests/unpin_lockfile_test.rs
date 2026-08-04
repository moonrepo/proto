use proto_core::test_utils::*;
use proto_core::{Id, LockRecord, ProtoLock, UnresolvedVersionSpec, VersionSpec};
use starbase_sandbox::predicates::prelude::*;
use std::fs;
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

mod unpin_lockfile {
    use super::*;

    #[test]
    fn removes_records_when_unpinning() {
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
                cmd.arg("unpin").arg("protostar");
            })
            .success();

        let config = fs::read_to_string(sandbox.path().join(".prototools")).unwrap();

        assert!(predicate::str::contains("protostar").not().eval(&config));

        // The only record was removed, so the lockfile is deleted
        assert!(!sandbox.path().join(".protolock").exists());
    }

    #[test]
    fn keeps_records_for_other_tools() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(
            ".prototools",
            r#"protostar = "4.0.0"
moonbase = "3.2.1"

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
        lock.tools.insert(
            Id::raw("moonbase"),
            vec![create_lockfile_record("3.2.1", "3.2.1")],
        );
        lock.path = sandbox.path().join(".protolock");
        lock.save().unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("unpin").arg("protostar");
            })
            .success();

        let lock = ProtoLock::load_from(sandbox.path()).unwrap();

        assert!(!lock.tools.contains_key(&Id::raw("protostar")));
        assert!(lock.tools.contains_key(&Id::raw("moonbase")));
    }
}
