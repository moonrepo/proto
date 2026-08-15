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

    mod env_mode {
        use super::*;

        fn create_env_lockfile_sandbox() -> ProtoSandbox {
            let sandbox = create_lockfile_sandbox();
            sandbox.create_file(".prototools.production", r#"protostar = "5.0.0""#);

            let mut lock = ProtoLock::default();
            lock.tools.insert(
                Id::raw("protostar"),
                vec![LockRecord {
                    spec: Some(UnresolvedVersionSpec::parse("5.0.0").unwrap()),
                    version: Some(VersionSpec::parse("5.0.0").unwrap()),
                    os: Some(SystemOS::default()),
                    arch: Some(SystemArch::default()),
                    ..Default::default()
                }],
            );
            lock.path = sandbox.path().join(".protolock.production");
            lock.save().unwrap();

            sandbox
        }

        #[test]
        fn renders_env_lockfile_contents() {
            let sandbox = create_env_lockfile_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("debug")
                    .arg("config")
                    .env("PROTO_ENV", "production");
            });

            let output = assert.output();

            // Both lockfiles are rendered
            assert!(predicate::str::contains(r#"spec = "4.0.0""#).eval(&output));
            assert!(predicate::str::contains(r#"spec = "5.0.0""#).eval(&output));
        }

        #[test]
        fn doesnt_render_env_lockfile_when_env_not_active() {
            let sandbox = create_env_lockfile_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("debug").arg("config");
            });

            let output = assert.output();

            assert!(predicate::str::contains(r#"spec = "4.0.0""#).eval(&output));
            assert!(
                predicate::str::contains(r#"spec = "5.0.0""#)
                    .not()
                    .eval(&output)
            );

            // And the inactive lockfile is left alone
            assert!(sandbox.path().join(".protolock.production").exists());
        }

        #[test]
        fn includes_env_locks_in_json() {
            let sandbox = create_env_lockfile_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("debug")
                    .arg("config")
                    .arg("--json")
                    .env("PROTO_ENV", "production");
            });

            let output = assert.output();

            assert!(predicate::str::contains(".protolock.production").eval(&output));
            assert!(predicate::str::contains(r#""spec": "4.0.0""#).eval(&output));
            assert!(predicate::str::contains(r#""spec": "5.0.0""#).eval(&output));
        }
    }
}
