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

mod debug_config_lockfile {
    use super::*;

    #[test]
    fn renders_lockfile_contents() {
        let sandbox = create_lockfile_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("debug").arg("config");
        });

        let output = assert.output();

        // Section titles are truncated to the console width, so assert
        // against the lockfile contents instead of its file path
        assert!(predicate::str::contains("[[tools.protostar]]").eval(&output));
        assert!(predicate::str::contains(r#"spec = "4.0.0""#).eval(&output));
    }

    #[test]
    fn doesnt_render_lockfile_when_disabled() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".prototools", r#"protostar = "4.0.0""#);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("debug").arg("config");
        });

        let output = assert.output();

        assert!(
            predicate::str::contains("[[tools.protostar]]")
                .not()
                .eval(&output)
        );
    }

    #[test]
    fn includes_locks_in_json() {
        let sandbox = create_lockfile_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("debug").arg("config").arg("--json");
        });

        let output = assert.output();

        assert!(predicate::str::contains(r#""locks""#).eval(&output));
        assert!(predicate::str::contains(".protolock").eval(&output));
    }
}
