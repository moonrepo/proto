use proto_core::test_utils::*;
use proto_core::{Id, LockRecord, ProtoLock, UnresolvedVersionSpec, VersionSpec};
use starbase_sandbox::predicates::prelude::*;
use system_env::{SystemArch, SystemOS};

fn create_lockfile_sandbox() -> ProtoSandbox {
    let sandbox = create_empty_proto_sandbox();
    sandbox.create_file(
        ".prototools",
        r#"protostar = "4.0.0"

[settings]
lockfile = true
"#,
    );

    let mut lock = ProtoLock::default();
    lock.tools.insert(
        Id::raw("protostar"),
        vec![LockRecord {
            spec: Some(UnresolvedVersionSpec::parse("4.0.0").unwrap()),
            version: Some(VersionSpec::parse("4.0.0").unwrap()),
            os: Some(SystemOS::default()),
            arch: Some(SystemArch::default()),
            ..Default::default()
        }],
    );
    lock.path = sandbox.path().join(".protolock");
    lock.save().unwrap();

    sandbox
}

mod versions_lockfile {
    use super::*;

    #[test]
    fn shows_locked_label() {
        let sandbox = create_lockfile_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("versions").arg("protostar");
        });

        let output = assert.output();

        assert!(predicate::str::contains("locked").eval(&output));
    }

    #[test]
    fn doesnt_show_locked_label_without_lockfile() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".prototools", r#"protostar = "4.0.0""#);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("versions").arg("protostar");
        });

        let output = assert.output();

        assert!(predicate::str::contains("locked").not().eval(&output));
    }

    #[test]
    fn includes_locked_in_json() {
        let sandbox = create_lockfile_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("versions").arg("protostar").arg("--json");
        });

        let output = assert.output();

        assert!(predicate::str::contains(r#""locked": true"#).eval(&output));
    }
}
