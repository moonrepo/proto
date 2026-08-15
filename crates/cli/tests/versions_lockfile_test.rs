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

    #[test]
    fn shows_locked_label_from_env_lockfile() {
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

        // The env config takes precedence, so only its lockfile is used
        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("versions")
                .arg("protostar")
                .arg("--json")
                .env("PROTO_ENV", "production");
        });

        let output: serde_json::Value = serde_json::from_str(&assert.stdout()).unwrap();
        let versions = output["versions"].as_array().unwrap();
        let locked = versions
            .iter()
            .filter(|item| item["locked"].as_bool().unwrap())
            .map(|item| item["version"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>();

        assert_eq!(locked, vec!["5.0.0".to_owned()]);

        // Otherwise the base lockfile is used
        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("versions").arg("protostar").arg("--json");
        });

        let output: serde_json::Value = serde_json::from_str(&assert.stdout()).unwrap();
        let versions = output["versions"].as_array().unwrap();
        let locked = versions
            .iter()
            .filter(|item| item["locked"].as_bool().unwrap())
            .map(|item| item["version"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>();

        assert_eq!(locked, vec!["4.0.0".to_owned()]);
    }
}
