use proto_core::test_utils::*;
use proto_core::{Id, LockRecord, ProtoLock, UnresolvedVersionSpec, VersionSpec};
use starbase_sandbox::predicates::prelude::*;
use std::path::Path;
use system_env::{SystemArch, SystemOS};

fn create_lockfile(path: &Path, spec: &str, version: &str) {
    let mut lock = ProtoLock::default();
    lock.tools.insert(
        Id::raw("protostar"),
        vec![LockRecord {
            spec: Some(UnresolvedVersionSpec::parse(spec).unwrap()),
            version: Some(VersionSpec::parse(version).unwrap()),
            os: Some(SystemOS::default()),
            arch: Some(SystemArch::default()),
            ..Default::default()
        }],
    );
    lock.path = path.to_path_buf();
    lock.save().unwrap();
}

// The `bin` command detects and resolves a version the same way
// `run` does, so use it to verify lockfile resolution
mod bin_lockfile {
    use super::*;

    #[test]
    fn resolves_version_from_lockfile() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(
            ".prototools",
            r#"protostar = "^5.10"

[settings]
lockfile = true
"#,
        );
        create_lockfile(&sandbox.path().join(".protolock"), "^5.10", "5.10.5");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("5.10.5");
            })
            .success();

        // 5.10.15 is the latest, but the lockfile pins 5.10.5
        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("bin").arg("protostar");
        });

        assert.success().stdout(predicate::str::contains("5.10.5"));
    }

    #[test]
    fn resolves_version_from_env_lockfile() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(
            ".prototools",
            r#"protostar = "^5.10"

[settings]
lockfile = true
"#,
        );
        sandbox.create_file(".prototools.production", r#"protostar = "^5.10""#);
        create_lockfile(&sandbox.path().join(".protolock"), "^5.10", "5.10.5");
        create_lockfile(
            &sandbox.path().join(".protolock.production"),
            "^5.10",
            "5.10.0",
        );

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("5.10.5");
            })
            .success();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("5.10.0");
            })
            .success();

        // The env config takes precedence, so its lockfile is used
        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("bin")
                .arg("protostar")
                .env("PROTO_ENV", "production");
        });

        assert.success().stdout(predicate::str::contains("5.10.0"));

        // Otherwise the base lockfile is used
        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("bin").arg("protostar");
        });

        assert.success().stdout(predicate::str::contains("5.10.5"));
    }
}
